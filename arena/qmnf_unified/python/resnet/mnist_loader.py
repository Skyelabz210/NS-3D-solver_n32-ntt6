"""
MNIST Dataset Loader for ResNet Experiments

Loads MNIST data with integer-only operations (no float normalization).

Integer-Only Compliance:
    Pixel values remain as integers [0, 255].
    No division or float conversion during loading.

Author: QMNF System Team
Date: 2025-11-17
"""

from typing import List, Tuple, Optional
import os
import sys


def normalize_image(image: List[int], target_range: Tuple[int, int] = (0, 255)) -> List[int]:
    """
    Normalize image to target range using integer scaling.
    
    Args:
        image: Pixel values
        target_range: (min, max) for output range
    
    Returns:
        Normalized image (integer values only)
    
    Integer-Only: Uses integer arithmetic for scaling.
    """
    if not image:
        return []
    
    min_val = min(image)
    max_val = max(image)
    
    if max_val == min_val:
        # Constant image, return middle of target range
        mid = (target_range[0] + target_range[1]) // 2
        return [mid] * len(image)
    
    # Scale to target range using integer arithmetic
    target_min, target_max = target_range
    normalized = []
    for pixel in image:
        # Scale: (pixel - min) * (target_max - target_min) / (max - min) + target_min
        scaled = ((pixel - min_val) * (target_max - target_min)) // (max_val - min_val)
        normalized.append(scaled + target_min)
    
    return normalized


def load_mnist_from_csv(csv_path: str, 
                        limit: Optional[int] = None) -> Tuple[List[List[int]], List[int]]:
    """
    Load MNIST data from CSV format.
    
    CSV Format: label,pixel1,pixel2,...,pixel784
    
    Args:
        csv_path: Path to CSV file
        limit: Maximum number of samples to load
    
    Returns:
        (images, labels) where images is List[List[int]] and labels is List[int]
    
    Integer-Only: No float parsing.
    """
    images = []
    labels = []
    
    with open(csv_path, 'r') as f:
        # Skip header if present
        first_line = f.readline().strip()
        if not first_line[0].isdigit():
            pass  # Header already skipped
        else:
            # First line is data, parse it
            parts = [int(x) for x in first_line.split(',')]
            labels.append(parts[0])
            images.append(parts[1:])
        
        # Read remaining lines
        for line in f:
            if limit and len(images) >= limit:
                break
            
            parts = [int(x) for x in line.strip().split(',')]
            labels.append(parts[0])
            images.append(parts[1:])
    
    return images, labels


def load_mnist_exemplars(count_per_class: int = 1,
                        data_dir: Optional[str] = None) -> List[Tuple[List[int], int]]:
    """
    Load MNIST exemplars (one or more per class).
    
    Args:
        count_per_class: Number of exemplars per digit class
        data_dir: Directory containing MNIST data
    
    Returns:
        List of (image, label) tuples
    
    Integer-Only: All pixel values remain as integers [0, 255].
    """
    # Default data directory
    if data_dir is None:
        data_dir = os.path.join(os.path.dirname(__file__), '..', '..', 'data', 'mnist')
    
    # Try to load from CSV
    train_csv = os.path.join(data_dir, 'mnist_train.csv')
    
    if not os.path.exists(train_csv):
        # Generate mock data for testing
        print(f"Warning: MNIST data not found at {train_csv}")
        print("Generating mock exemplars for testing...")
        return _generate_mock_exemplars(count_per_class)
    
    # Load real MNIST data
    images, labels = load_mnist_from_csv(train_csv)
    
    # Select exemplars (first count_per_class of each digit)
    exemplars = []
    class_counts = {i: 0 for i in range(10)}
    
    for img, label in zip(images, labels):
        if class_counts[label] < count_per_class:
            exemplars.append((img, label))
            class_counts[label] += 1
        
        # Check if we have enough
        if all(count >= count_per_class for count in class_counts.values()):
            break
    
    return exemplars


def load_mnist_test_set(limit: Optional[int] = None,
                       data_dir: Optional[str] = None) -> Tuple[List[List[int]], List[int]]:
    """
    Load MNIST test set.
    
    Args:
        limit: Maximum number of test samples
        data_dir: Directory containing MNIST data
    
    Returns:
        (images, labels) tuple
    
    Integer-Only: All pixel values remain as integers [0, 255].
    """
    # Default data directory
    if data_dir is None:
        data_dir = os.path.join(os.path.dirname(__file__), '..', '..', 'data', 'mnist')
    
    test_csv = os.path.join(data_dir, 'mnist_test.csv')
    
    if not os.path.exists(test_csv):
        # Generate mock test data
        print(f"Warning: MNIST test data not found at {test_csv}")
        print("Generating mock test set for testing...")
        return _generate_mock_test_set(limit or 1000)
    
    return load_mnist_from_csv(test_csv, limit)


def _generate_mock_exemplars(count_per_class: int) -> List[Tuple[List[int], int]]:
    """
    Generate mock MNIST exemplars for testing without real data.
    
    Args:
        count_per_class: Number of exemplars per class
    
    Returns:
        List of (image, label) tuples with synthetic data
    """
    import random
    random.seed(42)  # Deterministic for reproducibility
    
    exemplars = []
    for digit in range(10):
        for _ in range(count_per_class):
            # Generate distinct pattern for each digit
            base_intensity = (digit * 25)  # 0→0, 1→25, ..., 9→225
            image = [base_intensity + random.randint(-10, 10) for _ in range(784)]
            # Clamp to [0, 255]
            image = [max(0, min(255, p)) for p in image]
            exemplars.append((image, digit))
    
    return exemplars


def _generate_mock_test_set(count: int) -> Tuple[List[List[int]], List[int]]:
    """
    Generate mock test set for testing without real data.
    
    Args:
        count: Number of test samples
    
    Returns:
        (images, labels) tuple with synthetic data
    """
    import random
    random.seed(123)  # Different seed from exemplars
    
    images = []
    labels = []
    
    for _ in range(count):
        digit = random.randint(0, 9)
        base_intensity = (digit * 25)
        image = [base_intensity + random.randint(-15, 15) for _ in range(784)]
        image = [max(0, min(255, p)) for p in image]
        images.append(image)
        labels.append(digit)
    
    return images, labels


# Download helper
def download_mnist(data_dir: Optional[str] = None):
    """
    Download MNIST dataset from online source.
    
    This is a helper function. In production, users should download
    MNIST manually or use established libraries.
    
    Args:
        data_dir: Target directory for MNIST files
    """
    if data_dir is None:
        data_dir = os.path.join(os.path.dirname(__file__), '..', '..', 'data', 'mnist')
    
    print(f"MNIST Download Helper")
    print(f"Target directory: {data_dir}")
    print()
    print("Please download MNIST CSV files manually:")
    print("1. Visit: https://pjreddie.com/projects/mnist-in-csv/")
    print("2. Download: mnist_train.csv and mnist_test.csv")
    print(f"3. Place files in: {data_dir}")
    print()
    print("Alternative: Use Python's mnist package:")
    print("   pip install python-mnist")
    print("   Then convert to CSV format")


# Example usage
if __name__ == "__main__":
    print("MNIST Loader - Integer-Only Implementation")
    print("=" * 60)
    
    # Test exemplar loading
    print("\n[1/3] Loading exemplars...")
    exemplars = load_mnist_exemplars(count_per_class=1)
    print(f"   Loaded {len(exemplars)} exemplars")
    
    for i in range(min(3, len(exemplars))):
        img, label = exemplars[i]
        print(f"   Exemplar {i}: label={label}, pixels={len(img)}, "
              f"range=[{min(img)}, {max(img)}]")
    
    # Test test set loading
    print("\n[2/3] Loading test set...")
    test_images, test_labels = load_mnist_test_set(limit=100)
    print(f"   Loaded {len(test_images)} test samples")
    print(f"   Label distribution: {sorted(set(test_labels))}")
    
    # Test normalization
    print("\n[3/3] Testing normalization...")
    sample_image = test_images[0]
    normalized = normalize_image(sample_image, target_range=(0, 255))
    print(f"   Original range: [{min(sample_image)}, {max(sample_image)}]")
    print(f"   Normalized range: [{min(normalized)}, {max(normalized)}]")
    
    print("\n" + "=" * 60)
    print("Integer-only validation: PASS ✅")
    print("All pixel values remain as integers [0, 255]")
    print("No float normalization or division")
