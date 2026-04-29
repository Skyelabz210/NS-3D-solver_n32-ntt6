"""
Residue-Native Neural Networks - Research Implementation

This package implements "Neural Networks as Number-Theoretic Constructions:
Exact Learning via Chinese Remainder Theorem" using integer-only arithmetic.

Modules:
    mnist_loader: MNIST dataset loading with integer-only compliance (Step 1)
    perturbation: Algorithm 5.1 - Residue Space Exploration (Step 2)
    templates: Algorithm 5.3 - Template Extraction (Step 3)
    classifier: Consensus-based classification (Step 4)
    train_oneshot: Algorithm 5.4 - Complete training pipeline (Step 5)
    resnet: High-level Python API (legacy wrapper)
    resnet_classifier: Alternative classifier implementation

Example - Quick Start:
    >>> from experiments.research.resnet import OneShotTrainer
    >>> trainer = OneShotTrainer(moduli=[127, 131, 137], delta=5)
    >>> trainer.load_data()
    >>> trainer.train()
    >>> accuracy = trainer.evaluate()

Example - Step-by-Step:
    >>> from experiments.research.resnet import (
    ...     load_mnist_exemplars, PerturbationEngine,
    ...     TemplateExtractor, ResNetClassifier
    ... )
    >>> exemplars = load_mnist_exemplars(count_per_class=1)
    >>> engine = PerturbationEngine([127, 131, 137], delta=5)
    >>> variants = [engine.generate_variants(img) for img, _ in exemplars]
    >>> extractor = TemplateExtractor([127, 131, 137])
    >>> templates = extractor.batch_extract_templates(variants)
    >>> classifier = ResNetClassifier([127, 131, 137])
    >>> for i, template in enumerate(templates):
    ...     classifier.add_template(i, template)
"""

# Legacy API (from resnet.py)
from .resnet import (
    ResNetClassifier as LegacyResNetClassifier,
)

# Data loading (Step 1)
from .mnist_loader import (
    load_mnist_exemplars,
    load_mnist_test_set,
    normalize_image,
)

# Algorithm 5.1: Residue Space Exploration (Step 2)
from .perturbation import (
    PerturbationEngine,
    generate_synthetic_set,
    batch_generate_synthetic_sets,
)

# Algorithm 5.3: Template Extraction (Step 3)
from .templates import (
    TemplateExtractor,
    extract_template,
    batch_extract_templates,
)

# Consensus Classifier (Step 4)
from .classifier import (
    ResNetClassifier,
    ConsensusMetric,
)

# Algorithm 5.4: One-Shot Training (Step 5)
from .train_oneshot import (
    OneShotTrainer,
)

__all__ = [
    # Data loading
    'load_mnist_exemplars',
    'load_mnist_test_set',
    'normalize_image',
    # Perturbation (Algorithm 5.1)
    'PerturbationEngine',
    'generate_synthetic_set',
    'batch_generate_synthetic_sets',
    # Templates (Algorithm 5.3)
    'TemplateExtractor',
    'extract_template',
    'batch_extract_templates',
    # Classifier
    'ResNetClassifier',
    'ConsensusMetric',
    # Training (Algorithm 5.4)
    'OneShotTrainer',
    # Legacy
    'LegacyResNetClassifier',
]

__version__ = '0.2.0'
