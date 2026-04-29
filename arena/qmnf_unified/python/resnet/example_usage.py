#!/usr/bin/env python3
"""
MNIST ResNet Example Usage

Demonstrates how to load MNIST data and prepare it for ResNet training.

Integer-Only Compliance:
    All pixel values remain as integers [0, 255].
    No float conversion or normalization.

Author: QMNF System Team
Date: 2025-11-17
"""

import sys
from pathlib import Path

# Add project root to path
project_root = Path(__file__).parent.parent.parent.parent
sys.path.insert(0, str(project_root))

from experiments.research.resnet import (
    download_mnist,
    load_mnist_exemplars,
    load_mnist_test_set,
    normalize_image,
)


def main():
    """Main example workflow."""
    print("="*60)
    print("MNIST ResNet Example Usage")
    print("Integer-Only MNIST Data Loading")
    print("="*60)

    # Step 1: Download MNIST (instructions only)
    print("\n[Step 1] Download MNIST Dataset")
    print("-" * 60)
    download_mnist()

    # Step 2: Load exemplars (one-shot learning)
    print("\n[Step 2] Load Exemplars (One-Shot Learning)")
    print("-" * 60)
    exemplars = load_mnist_exemplars(count_per_class=1)
    print(f"Loaded {len(exemplars)} exemplars (1 per digit class)")

    for i, (image, label) in enumerate(exemplars[:3]):
        print(f"  Exemplar {i}: label={label}, pixels={len(image)}, "
              f"range=[{min(image)}, {max(image)}]")

    # Step 3: Load test set
    print("\n[Step 3] Load Test Set")
    print("-" * 60)
    test_images, test_labels = load_mnist_test_set(limit=100)
    print(f"Loaded {len(test_images)} test samples")

    # Count labels
    label_counts = {}
    for label in test_labels:
        label_counts[label] = label_counts.get(label, 0) + 1

    print(f"Label distribution: {dict(sorted(label_counts.items()))}")

    # Step 4: Normalize (optional, integer-only)
    print("\n[Step 4] Normalize Image (Integer-Only)")
    print("-" * 60)
    sample_image = test_images[0]
    print(f"Original range: [{min(sample_image)}, {max(sample_image)}]")

    # Normalize to [0, 255] (identity)
    normalized_255 = normalize_image(sample_image, target_range=(0, 255))
    print(f"Normalized [0, 255]: [{min(normalized_255)}, {max(normalized_255)}]")

    # Normalize to [0, 100] (scaled)
    normalized_100 = normalize_image(sample_image, target_range=(0, 100))
    print(f"Normalized [0, 100]: [{min(normalized_100)}, {max(normalized_100)}]")

    # Step 5: Prepare for ResNet
    print("\n[Step 5] Prepare for ResNet")
    print("-" * 60)
    print("Images are ready for ResNet processing:")
    print(f"  - Format: List[int] with length 784 (28×28 pixels)")
    print(f"  - Pixel range: [0, 255] (integer-only)")
    print(f"  - Labels: [0, 9] (integer digit classes)")
    print(f"  - Exemplars: {len(exemplars)} (for similarity comparison)")
    print(f"  - Test samples: {len(test_images)} (for evaluation)")

    # Step 6: Integer-only validation
    print("\n[Step 6] Integer-Only Validation")
    print("-" * 60)
    all_valid = True

    for i, (image, label) in enumerate(exemplars):
        # Check all pixels are integers
        if not all(isinstance(p, int) for p in image):
            print(f"  ❌ Exemplar {i}: Contains non-integer pixels")
            all_valid = False
            continue

        # Check pixel range
        if not all(0 <= p <= 255 for p in image):
            print(f"  ❌ Exemplar {i}: Pixels out of range [0, 255]")
            all_valid = False
            continue

    if all_valid:
        print("  ✅ All exemplars pass integer-only validation")
        print("  ✅ No float contamination detected")
        print("  ✅ Ready for residue neural network training")

    print("\n" + "="*60)
    print("Example completed successfully!")
    print("="*60)


if __name__ == "__main__":
    main()
