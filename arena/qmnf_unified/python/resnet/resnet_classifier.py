"""
High-level Python API for Residue-Native Neural Networks

This module provides a convenient Python interface to the Rust-based ResNet
implementation using Chinese Remainder Theorem for exact integer computation.

Integer-Only Compliance:
    All operations use modular integer arithmetic via Rust FFI.
    No float contamination in core mathematical operations.

Example:
    >>> from experiments.research.resnet import ResNetClassifier
    >>> import numpy as np
    >>>
    >>> # Create classifier with 3 channels
    >>> classifier = ResNetClassifier(
    ...     moduli=[127, 131, 137],
    ...     architecture=[784, 128, 10],
    ...     perturbation_radius=5
    ... )
    >>>
    >>> # Train from 10 exemplars (one per digit)
    >>> exemplars = [np.random.randint(0, 256, 784, dtype=np.uint8) for _ in range(10)]
    >>> classifier.train_oneshot(exemplars)
    >>>
    >>> # Classify test image
    >>> test_image = np.random.randint(0, 256, 784, dtype=np.uint8)
    >>> predicted_class = classifier.classify(test_image)
    >>> print(f"Predicted: {predicted_class}")
"""

from typing import List, Tuple, Optional
import numpy as np

try:
    from hcvlang import ResNetArchitecture, ConsensusClassifier, OneShotLearner
except ImportError:
    # Fallback for development without compiled module
    print("Warning: hcvlang module not found. Please build with: cargo build --release --features python --lib")
    ResNetArchitecture = None
    ConsensusClassifier = None
    OneShotLearner = None


class ResNetClassifier:
    """
    High-level interface for Residue-Native Neural Network classification.

    This classifier uses Chinese Remainder Theorem to encode inputs across
    multiple coprime moduli, enabling exact integer arithmetic with no
    accumulated error during training and inference.

    Attributes:
        moduli: List of prime moduli for CRT channels
        architecture: Network layer sizes [input, hidden, output]
        perturbation_radius: Radius for synthetic data generation
        learner: Rust OneShotLearner instance
        classifier: Trained ConsensusClassifier (after training)

    Example:
        >>> classifier = ResNetClassifier(
        ...     moduli=[127, 131, 137],
        ...     architecture=[784, 128, 10]
        ... )
        >>> classifier.train_oneshot(mnist_exemplars)
        >>> prediction = classifier.classify(test_image)
    """

    def __init__(
        self,
        moduli: Optional[List[int]] = None,
        architecture: Optional[List[int]] = None,
        perturbation_radius: int = 5
    ):
        """
        Initialize ResNet classifier.

        Args:
            moduli: Prime moduli for CRT channels (default: [127, 131, 137])
            architecture: Layer sizes (default: [784, 128, 10] for MNIST)
            perturbation_radius: Radius for pixel perturbations (default: 5)

        Raises:
            ImportError: If hcvlang module not available
            ValueError: If moduli not coprime or architecture invalid
        """
        if OneShotLearner is None:
            raise ImportError(
                "hcvlang module not available. "
                "Build with: cd hcvlang && cargo build --release --features python --lib"
            )

        # Default configuration for MNIST
        self.moduli = moduli or [127, 131, 137]
        self.architecture = architecture or [784, 128, 10]
        self.perturbation_radius = perturbation_radius

        # Validate configuration
        self._validate_config()

        # Create Rust learner
        self.learner = OneShotLearner(
            self.moduli,
            self.architecture,
            self.perturbation_radius
        )

        # Classifier (created after training)
        self.classifier: Optional[ConsensusClassifier] = None

    def _validate_config(self):
        """Validate moduli and architecture configuration."""
        # Check moduli are positive
        if any(m <= 0 for m in self.moduli):
            raise ValueError("All moduli must be positive")

        # Check architecture has at least input and output
        if len(self.architecture) < 2:
            raise ValueError("Architecture must have at least input and output sizes")

        # Check perturbation radius is reasonable
        if self.perturbation_radius < 0 or self.perturbation_radius > 128:
            raise ValueError("Perturbation radius should be in [0, 128]")

    def train_oneshot(self, exemplars: List[np.ndarray]) -> None:
        """
        Train classifier from single exemplar per class.

        This method uses systematic perturbations to generate synthetic
        training data from each exemplar, then creates templates for
        consensus-based classification.

        Args:
            exemplars: List of exemplar images, one per class
                      Each should be numpy array of uint8 values
                      Shape: (input_size,) matching architecture[0]

        Raises:
            ValueError: If exemplar count doesn't match output classes
            TypeError: If exemplars not uint8 numpy arrays

        Example:
            >>> exemplars = [mnist_train[i] for i in range(10)]
            >>> classifier.train_oneshot(exemplars)
        """
        # Validate exemplars
        num_classes = self.architecture[-1]
        if len(exemplars) != num_classes:
            raise ValueError(
                f"Expected {num_classes} exemplars, got {len(exemplars)}"
            )

        # Convert to list of lists (Rust expects Vec<Vec<u8>>)
        exemplars_list = []
        for i, exemplar in enumerate(exemplars):
            # Convert to numpy array if not already
            arr = np.asarray(exemplar, dtype=np.uint8)

            # Flatten if needed
            arr = arr.flatten()

            # Check size
            if len(arr) != self.architecture[0]:
                raise ValueError(
                    f"Exemplar {i} has size {len(arr)}, "
                    f"expected {self.architecture[0]}"
                )

            # Convert to Python list for FFI
            exemplars_list.append(arr.tolist())

        # Train via Rust FFI
        self.classifier = self.learner.train(exemplars_list)

    def classify(self, image: np.ndarray) -> int:
        """
        Classify a single image.

        Args:
            image: Input image as numpy array (uint8)
                  Shape: (input_size,) matching architecture[0]

        Returns:
            Predicted class ID (0-indexed)

        Raises:
            ValueError: If classifier not trained or image wrong size

        Example:
            >>> prediction = classifier.classify(test_image)
            >>> print(f"Predicted digit: {prediction}")
        """
        if self.classifier is None:
            raise ValueError(
                "Classifier not trained. Call train_oneshot() first."
            )

        # Convert and validate input
        pixels = np.asarray(image, dtype=np.uint8).flatten()
        if len(pixels) != self.architecture[0]:
            raise ValueError(
                f"Image has size {len(pixels)}, "
                f"expected {self.architecture[0]}"
            )

        # Get architecture (need to reconstruct to call forward)
        # Create temporary architecture for encoding
        temp_arch = ResNetArchitecture(self.moduli, self.architecture)
        encoded = temp_arch.forward(pixels.tolist())

        # Classify via consensus
        prediction = self.classifier.classify(encoded, self.moduli)
        return prediction

    def get_consensus_scores(
        self,
        image: np.ndarray
    ) -> List[Tuple[int, int]]:
        """
        Get consensus scores for all classes.

        Returns rational scores as (numerator, denominator) pairs,
        maintaining exact integer arithmetic.

        Args:
            image: Input image as numpy array (uint8)

        Returns:
            List of (numerator, denominator) tuples, one per class
            Higher values indicate stronger match

        Example:
            >>> scores = classifier.get_consensus_scores(test_image)
            >>> for i, (num, den) in enumerate(scores):
            ...     print(f"Class {i}: {num}/{den} = {num/den:.4f}")
        """
        if self.classifier is None:
            raise ValueError(
                "Classifier not trained. Call train_oneshot() first."
            )

        # Convert and validate input
        pixels = np.asarray(image, dtype=np.uint8).flatten()
        if len(pixels) != self.architecture[0]:
            raise ValueError(
                f"Image has size {len(pixels)}, "
                f"expected {self.architecture[0]}"
            )

        # Encode via architecture
        temp_arch = ResNetArchitecture(self.moduli, self.architecture)
        encoded = temp_arch.forward(pixels.tolist())

        # Get consensus scores
        scores = self.classifier.get_consensus_scores(encoded, self.moduli)
        return scores

    def generate_synthetic_data(
        self,
        exemplar: np.ndarray
    ) -> List[np.ndarray]:
        """
        Generate synthetic training data via perturbations.

        This exposes the underlying perturbation mechanism for analysis
        and visualization.

        Args:
            exemplar: Single example image

        Returns:
            List of perturbed versions (as multi-channel representations)

        Example:
            >>> synthetic = classifier.generate_synthetic_data(exemplar)
            >>> print(f"Generated {len(synthetic)} variations")
        """
        pixels = np.asarray(exemplar, dtype=np.uint8).flatten()
        synthetic = self.learner.generate_synthetic_set(pixels.tolist())
        return synthetic

    def __repr__(self) -> str:
        """String representation of classifier."""
        trained = "trained" if self.classifier is not None else "untrained"
        return (
            f"ResNetClassifier("
            f"channels={len(self.moduli)}, "
            f"architecture={self.architecture}, "
            f"status={trained})"
        )


__all__ = ['ResNetClassifier']
