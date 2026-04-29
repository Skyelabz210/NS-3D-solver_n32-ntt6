"""
Ablation Studies for ResNet Architecture Validation (Table 3)

CLASSIFICATION: BENCHMARK/TIMING
Float usage: time.time() for performance measurement only.
Core mathematical operations use integer modular arithmetic.

This script runs comprehensive ablation experiments to validate architectural
choices for the Residue-Native Neural Network (ResNet) implementation.

Experiments:
1. Vary number of channels (k=2,3,4,5)
2. Vary perturbation radius (δ=3,5,7,10)
3. Alternative configurations (no consensus, different moduli)

Integer-Only Compliance:
    All core operations use modular integer arithmetic.
    Floats used only for timing metrics, not mathematical computation.

Author: QMNF System Team
Date: 2025-11-18
"""

import json
import time
from typing import List, Dict, Tuple, Any
from pathlib import Path
import sys
import os

# Add project root to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../../..'))

from experiments.research.resnet import ResNetClassifier, load_mnist_exemplars, load_mnist_test_set


class AblationStudy:
    """
    Manages ablation experiments for ResNet architecture validation.

    Runs systematic experiments varying:
    - Number of channels (k)
    - Perturbation radius (δ)
    - Moduli selection
    - Consensus mechanism
    """

    def __init__(self, num_exemplars: int = 10, test_size: int = 1000):
        """
        Initialize ablation study.

        Args:
            num_exemplars: Exemplars per class for training
            test_size: Number of test samples to evaluate
        """
        self.num_exemplars = num_exemplars
        self.test_size = test_size
        self.results: List[Dict[str, Any]] = []

        # Load MNIST data once (reuse across all experiments)
        print(f"Loading MNIST data...")
        print(f"  - Exemplars: {num_exemplars} per class")
        print(f"  - Test samples: {test_size}")

        self.exemplars = load_mnist_exemplars(count_per_class=1)
        self.test_images, self.test_labels = load_mnist_test_set(limit=test_size)

        print(f"✅ Loaded {len(self.exemplars)} exemplars")
        print(f"✅ Loaded {len(self.test_images)} test samples\n")

    def run_experiment(
        self,
        experiment_name: str,
        moduli: List[int],
        perturbation_radius: int,
        architecture: List[int] = [784, 128, 10]
    ) -> Dict[str, Any]:
        """
        Run single experiment configuration.

        Args:
            experiment_name: Descriptive name for this configuration
            moduli: List of prime moduli for CRT channels
            perturbation_radius: Radius for pixel perturbations
            architecture: Network layer sizes

        Returns:
            Dictionary with experiment results
        """
        print(f"Running: {experiment_name}")
        print(f"  Moduli: {moduli}")
        print(f"  Perturbation radius: {perturbation_radius}")
        print(f"  Channels (k): {len(moduli)}")

        # Create classifier
        classifier = ResNetClassifier(
            moduli=moduli,
            architecture=architecture,
            perturbation_radius=perturbation_radius
        )

        # Training phase
        start_train = time.time()
        exemplar_images = [img for img, label in self.exemplars]
        classifier.train_oneshot(exemplar_images)
        train_time = time.time() - start_train

        # Evaluation phase
        start_eval = time.time()
        predictions = classifier.batch_classify(self.test_images)
        eval_time = time.time() - start_eval

        # Compute accuracy
        correct = sum(1 for pred, true in zip(predictions, self.test_labels) if pred == true)
        accuracy = correct / len(self.test_labels)

        # Per-class accuracy
        class_correct = [0] * 10
        class_total = [0] * 10
        for pred, true in zip(predictions, self.test_labels):
            class_total[true] += 1
            if pred == true:
                class_correct[true] += 1

        class_accuracy = [
            (class_correct[i] / class_total[i] if class_total[i] > 0 else 0.0)
            for i in range(10)
        ]

        result = {
            'experiment_name': experiment_name,
            'moduli': moduli,
            'num_channels': len(moduli),
            'perturbation_radius': perturbation_radius,
            'architecture': architecture,
            'num_exemplars': len(self.exemplars),
            'test_size': len(self.test_labels),
            'accuracy': accuracy,
            'correct': correct,
            'train_time_sec': train_time,
            'eval_time_sec': eval_time,
            'avg_inference_time_ms': (eval_time / len(self.test_labels)) * 1000,
            'class_accuracy': class_accuracy,
        }

        print(f"  ✅ Accuracy: {accuracy:.4f} ({correct}/{len(self.test_labels)})")
        print(f"  ⏱️  Train time: {train_time:.3f}s")
        print(f"  ⏱️  Eval time: {eval_time:.3f}s")
        print(f"  ⏱️  Avg inference: {result['avg_inference_time_ms']:.2f}ms")
        print()

        return result

    def vary_num_channels(self) -> List[Dict[str, Any]]:
        """
        Experiment 1: Vary number of channels (k).

        Configurations:
        - k=2: moduli=[127, 131]
        - k=3: moduli=[127, 131, 137] (baseline)
        - k=4: moduli=[127, 131, 137, 139]
        - k=5: moduli=[127, 131, 137, 139, 149]

        Returns:
            List of experiment results
        """
        print("=" * 70)
        print("EXPERIMENT 1: Varying Number of Channels (k)")
        print("=" * 70)
        print()

        configurations = [
            ("k=2 channels", [127, 131]),
            ("k=3 channels (baseline)", [127, 131, 137]),
            ("k=4 channels", [127, 131, 137, 139]),
            ("k=5 channels", [127, 131, 137, 139, 149]),
        ]

        results = []
        for name, moduli in configurations:
            result = self.run_experiment(
                experiment_name=name,
                moduli=moduli,
                perturbation_radius=5  # Baseline radius
            )
            results.append(result)
            self.results.append(result)

        return results

    def vary_perturbation_radius(self) -> List[Dict[str, Any]]:
        """
        Experiment 2: Vary perturbation radius (δ).

        Configurations:
        - δ=3
        - δ=5 (baseline)
        - δ=7
        - δ=10

        Returns:
            List of experiment results
        """
        print("=" * 70)
        print("EXPERIMENT 2: Varying Perturbation Radius (δ)")
        print("=" * 70)
        print()

        baseline_moduli = [127, 131, 137]  # k=3 baseline

        configurations = [
            ("δ=3", 3),
            ("δ=5 (baseline)", 5),
            ("δ=7", 7),
            ("δ=10", 10),
        ]

        results = []
        for name, radius in configurations:
            result = self.run_experiment(
                experiment_name=name,
                moduli=baseline_moduli,
                perturbation_radius=radius
            )
            results.append(result)
            self.results.append(result)

        return results

    def test_alternative_moduli(self) -> List[Dict[str, Any]]:
        """
        Experiment 3: Test alternative moduli selections.

        Configurations:
        - Baseline: [127, 131, 137]
        - Larger primes: [251, 257, 263]
        - Smaller primes: [101, 103, 107]
        - Mixed sizes: [127, 251, 103]

        Returns:
            List of experiment results
        """
        print("=" * 70)
        print("EXPERIMENT 3: Alternative Moduli Selections")
        print("=" * 70)
        print()

        configurations = [
            ("Baseline moduli", [127, 131, 137]),
            ("Larger primes", [251, 257, 263]),
            ("Smaller primes", [101, 103, 107]),
            ("Mixed sizes", [127, 251, 103]),
        ]

        results = []
        for name, moduli in configurations:
            result = self.run_experiment(
                experiment_name=name,
                moduli=moduli,
                perturbation_radius=5  # Baseline radius
            )
            results.append(result)
            self.results.append(result)

        return results

    def test_no_consensus(self) -> Dict[str, Any]:
        """
        Experiment 4: Direct residue comparison (no consensus).

        This tests the importance of multi-channel consensus by using
        only a single channel.

        Returns:
            Experiment result
        """
        print("=" * 70)
        print("EXPERIMENT 4: No Channel Consensus (Single Channel)")
        print("=" * 70)
        print()

        result = self.run_experiment(
            experiment_name="No consensus (k=1)",
            moduli=[127],  # Single channel
            perturbation_radius=5
        )

        self.results.append(result)
        return result

    def run_all_experiments(self) -> List[Dict[str, Any]]:
        """
        Run complete ablation study suite.

        Returns:
            List of all experiment results
        """
        print("\n")
        print("╔" + "=" * 68 + "╗")
        print("║" + " " * 15 + "ResNet Ablation Study - Table 3" + " " * 21 + "║")
        print("║" + " " * 68 + "║")
        print("║  Validating architectural choices for Residue-Native NNs     ║")
        print("╚" + "=" * 68 + "╝")
        print()

        # Run all experiment suites
        self.vary_num_channels()
        self.vary_perturbation_radius()
        self.test_alternative_moduli()
        self.test_no_consensus()

        return self.results

    def save_results(self, filename: str = "results_table3.json"):
        """
        Save results to JSON file.

        Args:
            filename: Output filename (in results directory)
        """
        output_dir = Path(__file__).parent / "results"
        output_dir.mkdir(exist_ok=True)

        output_path = output_dir / filename

        with open(output_path, 'w') as f:
            json.dump({
                'metadata': {
                    'num_exemplars': self.num_exemplars,
                    'test_size': self.test_size,
                    'timestamp': time.strftime('%Y-%m-%d %H:%M:%S'),
                },
                'experiments': self.results
            }, f, indent=2)

        print(f"\n✅ Results saved to: {output_path}")

    def generate_markdown_table(self) -> str:
        """
        Generate markdown table for Table 3 (publication format).

        Returns:
            Markdown-formatted table string
        """
        md = "# Ablation Study Results - Table 3\n\n"
        md += "## Experiment 1: Number of Channels (k)\n\n"
        md += "| Channels (k) | Moduli | Accuracy | Train Time (s) | Inference (ms) |\n"
        md += "|--------------|--------|----------|----------------|----------------|\n"

        for result in self.results:
            if 'channels' in result['experiment_name'].lower():
                md += f"| {result['num_channels']} | {result['moduli']} | "
                md += f"{result['accuracy']:.4f} | {result['train_time_sec']:.3f} | "
                md += f"{result['avg_inference_time_ms']:.2f} |\n"

        md += "\n## Experiment 2: Perturbation Radius (δ)\n\n"
        md += "| Radius (δ) | Accuracy | Train Time (s) | Inference (ms) |\n"
        md += "|------------|----------|----------------|----------------|\n"

        for result in self.results:
            if 'δ=' in result['experiment_name']:
                md += f"| {result['perturbation_radius']} | {result['accuracy']:.4f} | "
                md += f"{result['train_time_sec']:.3f} | "
                md += f"{result['avg_inference_time_ms']:.2f} |\n"

        md += "\n## Experiment 3: Alternative Moduli\n\n"
        md += "| Configuration | Moduli | Accuracy | Train Time (s) |\n"
        md += "|---------------|--------|----------|----------------|\n"

        for result in self.results:
            name = result['experiment_name']
            if 'moduli' in name.lower() or 'primes' in name.lower() or 'sizes' in name.lower():
                md += f"| {name} | {result['moduli']} | "
                md += f"{result['accuracy']:.4f} | {result['train_time_sec']:.3f} |\n"

        md += "\n## Experiment 4: No Consensus\n\n"
        md += "| Configuration | Channels | Accuracy | Notes |\n"
        md += "|---------------|----------|----------|-------|\n"

        for result in self.results:
            if 'consensus' in result['experiment_name'].lower():
                md += f"| {result['experiment_name']} | {result['num_channels']} | "
                md += f"{result['accuracy']:.4f} | Direct residue comparison |\n"

        md += "\n## Summary Statistics\n\n"
        md += f"- **Total Experiments**: {len(self.results)}\n"
        md += f"- **Test Set Size**: {self.test_size} samples\n"
        md += f"- **Exemplars**: {self.num_exemplars} per class\n"

        # Find best configuration
        best = max(self.results, key=lambda x: x['accuracy'])
        md += f"\n**Best Configuration**:\n"
        md += f"- Name: {best['experiment_name']}\n"
        md += f"- Accuracy: {best['accuracy']:.4f}\n"
        md += f"- Moduli: {best['moduli']}\n"
        md += f"- Perturbation Radius: {best['perturbation_radius']}\n"

        return md

    def save_markdown_table(self, filename: str = "results_table3.md"):
        """
        Save markdown table to file.

        Args:
            filename: Output filename (in results directory)
        """
        output_dir = Path(__file__).parent / "results"
        output_dir.mkdir(exist_ok=True)

        output_path = output_dir / filename

        markdown = self.generate_markdown_table()

        with open(output_path, 'w') as f:
            f.write(markdown)

        print(f"✅ Markdown table saved to: {output_path}")

    def print_summary(self):
        """Print summary of all experiments."""
        print("\n")
        print("╔" + "=" * 68 + "╗")
        print("║" + " " * 20 + "ABLATION STUDY SUMMARY" + " " * 25 + "║")
        print("╚" + "=" * 68 + "╝")
        print()

        print(f"Total Experiments: {len(self.results)}")
        print(f"Test Set Size: {self.test_size}")
        print(f"Exemplars: {self.num_exemplars} per class")
        print()

        # Sort by accuracy
        sorted_results = sorted(self.results, key=lambda x: x['accuracy'], reverse=True)

        print("Top 5 Configurations by Accuracy:")
        print("-" * 70)
        for i, result in enumerate(sorted_results[:5], 1):
            print(f"{i}. {result['experiment_name']}")
            print(f"   Accuracy: {result['accuracy']:.4f} "
                  f"({result['correct']}/{result['test_size']})")
            print(f"   Moduli: {result['moduli']}")
            print(f"   Perturbation: δ={result['perturbation_radius']}")
            print()

        # Insights
        print("\nKey Insights:")
        print("-" * 70)

        # Find optimal k
        channel_results = [r for r in self.results if 'channels' in r['experiment_name'].lower()]
        if channel_results:
            best_k = max(channel_results, key=lambda x: x['accuracy'])
            print(f"✓ Optimal number of channels: k={best_k['num_channels']} "
                  f"(accuracy: {best_k['accuracy']:.4f})")

        # Find optimal δ
        radius_results = [r for r in self.results if 'δ=' in r['experiment_name']]
        if radius_results:
            best_delta = max(radius_results, key=lambda x: x['accuracy'])
            print(f"✓ Optimal perturbation radius: δ={best_delta['perturbation_radius']} "
                  f"(accuracy: {best_delta['accuracy']:.4f})")

        # Consensus impact
        consensus_results = [r for r in self.results if 'consensus' in r['experiment_name'].lower()]
        if consensus_results and len(self.results) > len(consensus_results):
            no_consensus = consensus_results[0]['accuracy']
            baseline = next((r['accuracy'] for r in self.results
                           if 'baseline' in r['experiment_name'].lower()), None)
            if baseline:
                improvement = ((baseline - no_consensus) / no_consensus) * 100
                print(f"✓ Multi-channel consensus improvement: {improvement:.1f}% "
                      f"over single channel")

        print()


def main():
    """Main execution function."""
    # Initialize ablation study
    study = AblationStudy(num_exemplars=10, test_size=1000)

    # Run all experiments
    results = study.run_all_experiments()

    # Print summary
    study.print_summary()

    # Save results
    study.save_results("results_table3.json")
    study.save_markdown_table("results_table3.md")

    print("\n✅ Ablation study complete!")
    print(f"   Total experiments: {len(results)}")
    print(f"   Results saved to: experiments/research/resnet/results/")


if __name__ == "__main__":
    main()
