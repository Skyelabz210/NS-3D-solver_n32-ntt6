#!/usr/bin/env python3
"""
ResNet Experiment Results Visualization

Generates plots and visualizations from experiment JSON results.

Integer-Only Note: While visualization uses matplotlib (float-based),
all underlying data comes from integer-only ResNet computations.

Author: QMNF System Team
Date: 2025-11-17
"""

import json
import sys
import os
from pathlib import Path
from typing import Dict, List, Any
import argparse

# Optional: matplotlib for plotting (graceful degradation if not available)
try:
    import matplotlib.pyplot as plt
    import matplotlib
    matplotlib.use('Agg')  # Non-interactive backend
    HAS_MATPLOTLIB = True
except ImportError:
    HAS_MATPLOTLIB = False
    print("Warning: matplotlib not available. ASCII plots only.")


def load_experiment_results(results_file: str) -> Dict:
    """Load experiment results from JSON file."""
    with open(results_file, 'r') as f:
        return json.load(f)


def plot_training_loss(data: Dict, output_dir: str):
    """Plot training loss curve."""
    if 'deep_resnet' not in data or data['deep_resnet'] is None:
        print("No deep_resnet data found")
        return

    resnet_data = data['deep_resnet']
    train_loss = resnet_data.get('train_loss', [])

    if not train_loss:
        print("No training loss data available")
        return

    if HAS_MATPLOTLIB:
        plt.figure(figsize=(10, 6))
        plt.plot(range(1, len(train_loss) + 1), train_loss, 'b-o', linewidth=2)
        plt.xlabel('Epoch', fontsize=12)
        plt.ylabel('Training Loss', fontsize=12)
        plt.title('Deep ResNet Training Loss Curve', fontsize=14, fontweight='bold')
        plt.grid(True, alpha=0.3)
        plt.tight_layout()
        output_file = os.path.join(output_dir, 'training_loss.png')
        plt.savefig(output_file, dpi=150)
        plt.close()
        print(f"Saved: {output_file}")
    else:
        # ASCII plot
        print("\nTraining Loss Curve (ASCII):")
        print_ascii_plot(train_loss, "Epoch", "Loss")


def plot_one_shot_comparison(data: Dict, output_dir: str):
    """Plot one-shot vs multi-shot accuracy comparison."""
    if 'one_shot_comparison' not in data or data['one_shot_comparison'] is None:
        print("No one_shot_comparison data found")
        return

    comp_data = data['one_shot_comparison']
    results = comp_data.get('results', [])

    if not results:
        print("No comparison results available")
        return

    num_examples = [r['num_examples'] for r in results]
    test_accuracy = [r['test_accuracy'] * 100 for r in results]

    if HAS_MATPLOTLIB:
        plt.figure(figsize=(10, 6))
        plt.plot(num_examples, test_accuracy, 'g-o', linewidth=2, markersize=8)
        plt.xlabel('Examples per Class', fontsize=12)
        plt.ylabel('Test Accuracy (%)', fontsize=12)
        plt.title('One-Shot vs Multi-Shot Learning', fontsize=14, fontweight='bold')
        plt.grid(True, alpha=0.3)
        plt.axvline(x=1, color='r', linestyle='--', label='One-Shot', alpha=0.7)
        plt.legend(fontsize=10)
        plt.tight_layout()
        output_file = os.path.join(output_dir, 'one_shot_comparison.png')
        plt.savefig(output_file, dpi=150)
        plt.close()
        print(f"Saved: {output_file}")
    else:
        # ASCII plot
        print("\nOne-Shot vs Multi-Shot Accuracy (ASCII):")
        for examples, accuracy in zip(num_examples, test_accuracy):
            bar_length = int(accuracy / 2)  # Scale to fit
            bar = '█' * bar_length
            print(f"{examples:>3} examples: {bar} {accuracy:.1f}%")


def plot_adversarial_robustness(data: Dict, output_dir: str):
    """Plot adversarial robustness results."""
    if 'adversarial_robustness' not in data or data['adversarial_robustness'] is None:
        print("No adversarial_robustness data found")
        return

    adv_data = data['adversarial_robustness']
    results = adv_data.get('results', [])

    if not results:
        print("No adversarial results available")
        return

    perturbations = [r['perturbation_strength'] for r in results]
    clean_acc = [r['clean_accuracy'] * 100 for r in results]
    adv_acc = [r['adversarial_accuracy'] * 100 for r in results]

    if HAS_MATPLOTLIB:
        plt.figure(figsize=(10, 6))
        plt.plot(perturbations, clean_acc, 'b-s', label='Clean Accuracy', linewidth=2)
        plt.plot(perturbations, adv_acc, 'r-o', label='Adversarial Accuracy', linewidth=2)
        plt.xlabel('Perturbation Strength', fontsize=12)
        plt.ylabel('Accuracy (%)', fontsize=12)
        plt.title('Adversarial Robustness', fontsize=14, fontweight='bold')
        plt.legend(fontsize=10)
        plt.grid(True, alpha=0.3)
        plt.tight_layout()
        output_file = os.path.join(output_dir, 'adversarial_robustness.png')
        plt.savefig(output_file, dpi=150)
        plt.close()
        print(f"Saved: {output_file}")
    else:
        # ASCII comparison
        print("\nAdversarial Robustness (ASCII):")
        print(f"{'Perturb':<10} {'Clean%':<10} {'Adversarial%':<15} {'Drop%':<10}")
        print("-" * 50)
        for r in results:
            drop = (r['clean_accuracy'] - r['adversarial_accuracy']) * 100
            print(f"{r['perturbation_strength']:<10} "
                  f"{r['clean_accuracy']*100:<10.1f} "
                  f"{r['adversarial_accuracy']*100:<15.1f} "
                  f"{drop:<10.1f}")


def plot_active_learning(data: Dict, output_dir: str):
    """Plot active learning curves."""
    if 'active_learning' not in data or data['active_learning'] is None:
        print("No active_learning data found")
        return

    active_data = data['active_learning']
    rounds = active_data.get('rounds', [])

    if not rounds:
        print("No active learning rounds available")
        return

    round_nums = [r['round'] for r in rounds]
    accuracy = [r['accuracy'] * 100 for r in rounds]
    uncertainty = [r['avg_uncertainty'] for r in rounds]

    if HAS_MATPLOTLIB:
        fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

        # Accuracy curve
        ax1.plot(round_nums, accuracy, 'b-o', linewidth=2)
        ax1.set_xlabel('Query Round', fontsize=12)
        ax1.set_ylabel('Test Accuracy (%)', fontsize=12)
        ax1.set_title('Active Learning: Accuracy', fontsize=14, fontweight='bold')
        ax1.grid(True, alpha=0.3)

        # Uncertainty curve
        ax2.plot(round_nums, uncertainty, 'r-s', linewidth=2)
        ax2.set_xlabel('Query Round', fontsize=12)
        ax2.set_ylabel('Average Uncertainty', fontsize=12)
        ax2.set_title('Active Learning: Uncertainty', fontsize=14, fontweight='bold')
        ax2.grid(True, alpha=0.3)

        plt.tight_layout()
        output_file = os.path.join(output_dir, 'active_learning.png')
        plt.savefig(output_file, dpi=150)
        plt.close()
        print(f"Saved: {output_file}")
    else:
        # ASCII plot
        print("\nActive Learning Progress (ASCII):")
        print(f"{'Round':<8} {'Accuracy%':<12} {'Uncertainty':<12}")
        print("-" * 35)
        for r in rounds:
            print(f"{r['round']:<8} {r['accuracy']*100:<12.1f} {r['avg_uncertainty']:<12.2f}")


def plot_performance_benchmarks(data: Dict, output_dir: str):
    """Plot performance benchmark results."""
    if 'performance_benchmark' not in data or data['performance_benchmark'] is None:
        print("No performance_benchmark data found")
        return

    bench_data = data['performance_benchmark']
    results = bench_data.get('results', [])

    if not results:
        print("No benchmark results available")
        return

    # Group by architecture
    arch_groups = {}
    for r in results:
        arch_str = str(r['architecture'])
        if arch_str not in arch_groups:
            arch_groups[arch_str] = []
        arch_groups[arch_str].append(r)

    if HAS_MATPLOTLIB:
        fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

        # Training throughput
        for arch_str, arch_results in arch_groups.items():
            batch_sizes = [r['batch_size'] for r in arch_results]
            throughput = [r['training_throughput'] for r in arch_results]
            ax1.plot(batch_sizes, throughput, '-o', label=arch_str, linewidth=2)

        ax1.set_xlabel('Batch Size', fontsize=12)
        ax1.set_ylabel('Training Throughput (examples/sec)', fontsize=12)
        ax1.set_title('Training Performance', fontsize=14, fontweight='bold')
        ax1.legend(fontsize=8)
        ax1.grid(True, alpha=0.3)

        # Inference latency
        for arch_str, arch_results in arch_groups.items():
            moduli = [r['num_moduli'] for r in arch_results]
            latency = [r['inference_latency_us'] for r in arch_results]
            ax2.plot(moduli, latency, '-s', label=arch_str, linewidth=2)

        ax2.set_xlabel('Number of Moduli', fontsize=12)
        ax2.set_ylabel('Inference Latency (μs)', fontsize=12)
        ax2.set_title('Inference Performance', fontsize=14, fontweight='bold')
        ax2.legend(fontsize=8)
        ax2.grid(True, alpha=0.3)

        plt.tight_layout()
        output_file = os.path.join(output_dir, 'performance_benchmarks.png')
        plt.savefig(output_file, dpi=150)
        plt.close()
        print(f"Saved: {output_file}")
    else:
        # ASCII summary
        print("\nPerformance Benchmarks (ASCII):")
        print(f"{'Batch':<8} {'Moduli':<8} {'Train/sec':<12} {'Infer μs':<12}")
        print("-" * 45)
        for r in results[:10]:  # Show first 10
            print(f"{r['batch_size']:<8} {r['num_moduli']:<8} "
                  f"{r['training_throughput']:<12.0f} "
                  f"{r['inference_latency_us']:<12.0f}")


def print_ascii_plot(values: List[float], xlabel: str, ylabel: str):
    """Print ASCII art plot for when matplotlib is unavailable."""
    if not values:
        return

    # Normalize to 0-50 range for display
    min_val = min(values)
    max_val = max(values)
    range_val = max_val - min_val

    if range_val == 0:
        range_val = 1

    print(f"\n{ylabel} vs {xlabel}")
    print("-" * 60)

    for i, val in enumerate(values):
        normalized = int(((val - min_val) / range_val) * 50)
        bar = '█' * normalized
        print(f"{i+1:>3}: {bar} {val:.4f}")

    print("-" * 60)


def generate_summary_report(data: Dict, output_dir: str):
    """Generate text summary report."""
    output_file = os.path.join(output_dir, 'SUMMARY_REPORT.txt')

    with open(output_file, 'w') as f:
        f.write("=" * 70 + "\n")
        f.write("ResNet Comprehensive Experiment Summary\n")
        f.write("=" * 70 + "\n\n")

        f.write(f"Timestamp: {data.get('timestamp', 'N/A')}\n")
        f.write(f"Total Time: {data.get('total_time_ms', 0) / 1000:.2f} seconds\n\n")

        summary = data.get('summary', {})
        f.write(f"Experiments Run: {summary.get('experiments_run', 0)}\n")
        f.write(f"Passed: {summary.get('experiments_passed', 0)}\n")
        f.write(f"Failed: {summary.get('experiments_failed', 0)}\n\n")

        f.write("Key Findings:\n")
        for i, finding in enumerate(summary.get('key_findings', []), 1):
            f.write(f"{i}. {finding}\n")

        f.write("\n" + "=" * 70 + "\n")

    print(f"Saved: {output_file}")


def main():
    parser = argparse.ArgumentParser(
        description='Visualize ResNet experiment results'
    )
    parser.add_argument(
        'results_file',
        help='Path to experiment results JSON file'
    )
    parser.add_argument(
        '--output-dir',
        default='experiments/research/resnet/plots',
        help='Output directory for plots (default: plots/)'
    )

    args = parser.parse_args()

    # Validate input file
    if not os.path.exists(args.results_file):
        print(f"Error: Results file not found: {args.results_file}")
        sys.exit(1)

    # Create output directory
    os.makedirs(args.output_dir, exist_ok=True)

    # Load data
    print(f"Loading results from: {args.results_file}")
    data = load_experiment_results(args.results_file)

    # Generate visualizations
    print("\nGenerating visualizations...")
    plot_training_loss(data, args.output_dir)
    plot_one_shot_comparison(data, args.output_dir)
    plot_adversarial_robustness(data, args.output_dir)
    plot_active_learning(data, args.output_dir)
    plot_performance_benchmarks(data, args.output_dir)

    # Generate summary
    generate_summary_report(data, args.output_dir)

    print(f"\n✅ All visualizations saved to: {args.output_dir}")


if __name__ == "__main__":
    main()
