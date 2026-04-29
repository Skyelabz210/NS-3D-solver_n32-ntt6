"""
Visualization Script for ResNet Ablation Study Results

Generates charts and plots to illustrate key findings from Table 3 ablation experiments.

Author: QMNF System Team
Date: 2025-11-18
"""

import json
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path


def load_results(filename: str = "results_table3.json"):
    """Load ablation study results from JSON file."""
    results_dir = Path(__file__).parent / "results"
    filepath = results_dir / filename

    with open(filepath, 'r') as f:
        data = json.load(f)

    return data['experiments']


def plot_channels_vs_accuracy(results):
    """Plot number of channels vs accuracy."""
    # Filter channel experiments
    channel_results = [r for r in results if 'channels' in r['experiment_name'].lower()]
    channel_results.sort(key=lambda x: x['num_channels'])

    channels = [r['num_channels'] for r in channel_results]
    accuracies = [r['accuracy'] * 100 for r in channel_results]
    inference_times = [r['avg_inference_time_ms'] for r in channel_results]

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5))

    # Accuracy plot
    ax1.plot(channels, accuracies, 'bo-', linewidth=2, markersize=10)
    ax1.set_xlabel('Number of Channels (k)', fontsize=12, fontweight='bold')
    ax1.set_ylabel('Accuracy (%)', fontsize=12, fontweight='bold')
    ax1.set_title('Classification Accuracy vs Number of Channels', fontsize=14, fontweight='bold')
    ax1.grid(True, alpha=0.3)
    ax1.set_xticks(channels)
    ax1.set_ylim([50, 105])

    # Add value labels
    for i, (c, acc) in enumerate(zip(channels, accuracies)):
        ax1.annotate(f'{acc:.1f}%',
                    xy=(c, acc),
                    xytext=(0, 10),
                    textcoords='offset points',
                    ha='center',
                    fontsize=10,
                    fontweight='bold')

    # Inference time plot
    ax2.plot(channels, inference_times, 'ro-', linewidth=2, markersize=10)
    ax2.set_xlabel('Number of Channels (k)', fontsize=12, fontweight='bold')
    ax2.set_ylabel('Inference Time (ms)', fontsize=12, fontweight='bold')
    ax2.set_title('Inference Time vs Number of Channels', fontsize=14, fontweight='bold')
    ax2.grid(True, alpha=0.3)
    ax2.set_xticks(channels)

    # Add value labels
    for i, (c, time) in enumerate(zip(channels, inference_times)):
        ax2.annotate(f'{time:.2f}ms',
                    xy=(c, time),
                    xytext=(0, 10),
                    textcoords='offset points',
                    ha='center',
                    fontsize=10,
                    fontweight='bold')

    plt.tight_layout()

    # Save figure
    output_path = Path(__file__).parent / "results" / "channels_analysis.png"
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"✅ Saved: {output_path}")
    plt.close()


def plot_perturbation_analysis(results):
    """Plot perturbation radius analysis."""
    # Filter perturbation experiments
    perturb_results = [r for r in results if 'δ=' in r['experiment_name']]
    perturb_results.sort(key=lambda x: x['perturbation_radius'])

    radii = [r['perturbation_radius'] for r in perturb_results]
    accuracies = [r['accuracy'] * 100 for r in perturb_results]
    train_times = [r['train_time_sec'] * 1000 for r in perturb_results]  # Convert to ms

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5))

    # Accuracy plot
    ax1.plot(radii, accuracies, 'go-', linewidth=2, markersize=10)
    ax1.set_xlabel('Perturbation Radius (δ)', fontsize=12, fontweight='bold')
    ax1.set_ylabel('Accuracy (%)', fontsize=12, fontweight='bold')
    ax1.set_title('Classification Accuracy vs Perturbation Radius', fontsize=14, fontweight='bold')
    ax1.grid(True, alpha=0.3)
    ax1.set_xticks(radii)
    ax1.set_ylim([98, 100])

    # Add value labels
    for i, (r, acc) in enumerate(zip(radii, accuracies)):
        ax1.annotate(f'{acc:.1f}%',
                    xy=(r, acc),
                    xytext=(0, -20),
                    textcoords='offset points',
                    ha='center',
                    fontsize=10,
                    fontweight='bold')

    # Training time plot
    ax2.plot(radii, train_times, 'mo-', linewidth=2, markersize=10)
    ax2.set_xlabel('Perturbation Radius (δ)', fontsize=12, fontweight='bold')
    ax2.set_ylabel('Training Time (ms)', fontsize=12, fontweight='bold')
    ax2.set_title('Training Time vs Perturbation Radius', fontsize=14, fontweight='bold')
    ax2.grid(True, alpha=0.3)
    ax2.set_xticks(radii)

    # Add value labels
    for i, (r, time) in enumerate(zip(radii, train_times)):
        ax2.annotate(f'{time:.1f}ms',
                    xy=(r, time),
                    xytext=(0, 10),
                    textcoords='offset points',
                    ha='center',
                    fontsize=10,
                    fontweight='bold')

    plt.tight_layout()

    # Save figure
    output_path = Path(__file__).parent / "results" / "perturbation_analysis.png"
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"✅ Saved: {output_path}")
    plt.close()


def plot_moduli_comparison(results):
    """Plot alternative moduli comparison."""
    # Filter moduli experiments
    moduli_results = [r for r in results
                     if any(x in r['experiment_name'].lower()
                           for x in ['baseline moduli', 'larger primes', 'smaller primes', 'mixed sizes'])]

    names = [r['experiment_name'] for r in moduli_results]
    accuracies = [r['accuracy'] * 100 for r in moduli_results]
    products = [np.prod(r['moduli']) for r in moduli_results]

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    # Accuracy comparison
    colors = ['#1f77b4', '#2ca02c', '#d62728', '#ff7f0e']
    bars1 = ax1.bar(range(len(names)), accuracies, color=colors, alpha=0.7, edgecolor='black', linewidth=1.5)
    ax1.set_xlabel('Configuration', fontsize=12, fontweight='bold')
    ax1.set_ylabel('Accuracy (%)', fontsize=12, fontweight='bold')
    ax1.set_title('Accuracy by Moduli Configuration', fontsize=14, fontweight='bold')
    ax1.set_xticks(range(len(names)))
    ax1.set_xticklabels([n.replace(' moduli', '').replace(' primes', '') for n in names],
                        rotation=15, ha='right')
    ax1.set_ylim([70, 105])
    ax1.grid(True, alpha=0.3, axis='y')

    # Add value labels
    for i, (bar, acc) in enumerate(zip(bars1, accuracies)):
        ax1.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 1,
                f'{acc:.1f}%',
                ha='center', va='bottom', fontsize=10, fontweight='bold')

    # Product vs accuracy scatter
    ax2.scatter(products, accuracies, s=200, c=colors, alpha=0.7, edgecolors='black', linewidth=2)
    ax2.set_xlabel('CRT Product (M = m₁ × m₂ × m₃)', fontsize=12, fontweight='bold')
    ax2.set_ylabel('Accuracy (%)', fontsize=12, fontweight='bold')
    ax2.set_title('Accuracy vs CRT Product Range', fontsize=14, fontweight='bold')
    ax2.set_xscale('log')
    ax2.grid(True, alpha=0.3)

    # Add labels
    for i, (prod, acc, name) in enumerate(zip(products, accuracies, names)):
        label = name.replace(' moduli', '').replace(' primes', '')
        ax2.annotate(label,
                    xy=(prod, acc),
                    xytext=(10, -5),
                    textcoords='offset points',
                    fontsize=9,
                    bbox=dict(boxstyle='round,pad=0.3', facecolor=colors[i], alpha=0.3))

    plt.tight_layout()

    # Save figure
    output_path = Path(__file__).parent / "results" / "moduli_comparison.png"
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"✅ Saved: {output_path}")
    plt.close()


def plot_consensus_impact(results):
    """Plot impact of multi-channel consensus."""
    # Get single channel and baseline
    single = next(r for r in results if r['num_channels'] == 1)
    baseline = next(r for r in results if 'baseline' in r['experiment_name'].lower() and r['num_channels'] == 3)

    # Per-class accuracy comparison
    classes = list(range(10))
    single_acc = [acc * 100 for acc in single['class_accuracy']]
    baseline_acc = [acc * 100 for acc in baseline['class_accuracy']]

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    # Per-class comparison
    x = np.arange(len(classes))
    width = 0.35

    bars1 = ax1.bar(x - width/2, single_acc, width, label='k=1 (No Consensus)',
                   color='#d62728', alpha=0.7, edgecolor='black', linewidth=1)
    bars2 = ax1.bar(x + width/2, baseline_acc, width, label='k=3 (Consensus)',
                   color='#2ca02c', alpha=0.7, edgecolor='black', linewidth=1)

    ax1.set_xlabel('Digit Class', fontsize=12, fontweight='bold')
    ax1.set_ylabel('Accuracy (%)', fontsize=12, fontweight='bold')
    ax1.set_title('Per-Class Accuracy: Single Channel vs Multi-Channel', fontsize=14, fontweight='bold')
    ax1.set_xticks(x)
    ax1.set_xticklabels(classes)
    ax1.legend(fontsize=11)
    ax1.grid(True, alpha=0.3, axis='y')
    ax1.set_ylim([0, 110])

    # Overall accuracy comparison
    configs = ['k=1\n(No Consensus)', 'k=3\n(Consensus)']
    overall_acc = [single['accuracy'] * 100, baseline['accuracy'] * 100]

    bars = ax2.bar(configs, overall_acc, color=['#d62728', '#2ca02c'],
                  alpha=0.7, edgecolor='black', linewidth=2, width=0.6)
    ax2.set_ylabel('Overall Accuracy (%)', fontsize=12, fontweight='bold')
    ax2.set_title('Consensus Mechanism Impact', fontsize=14, fontweight='bold')
    ax2.set_ylim([0, 110])
    ax2.grid(True, alpha=0.3, axis='y')

    # Add value labels
    for bar, acc in zip(bars, overall_acc):
        ax2.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 2,
                f'{acc:.1f}%',
                ha='center', va='bottom', fontsize=14, fontweight='bold')

    # Add improvement annotation
    improvement = baseline['accuracy'] - single['accuracy']
    ax2.annotate(f'+{improvement*100:.1f}%\nimprovement',
                xy=(1, overall_acc[1]/2),
                ha='center', va='center',
                fontsize=13, fontweight='bold',
                bbox=dict(boxstyle='round,pad=0.5', facecolor='yellow', alpha=0.3))

    plt.tight_layout()

    # Save figure
    output_path = Path(__file__).parent / "results" / "consensus_impact.png"
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"✅ Saved: {output_path}")
    plt.close()


def plot_accuracy_speed_tradeoff(results):
    """Plot accuracy vs inference speed tradeoff."""
    # Get all channel configurations
    channel_results = [r for r in results if 'channels' in r['experiment_name'].lower()]

    accuracies = [r['accuracy'] * 100 for r in channel_results]
    inference_times = [r['avg_inference_time_ms'] for r in channel_results]
    channels = [r['num_channels'] for r in channel_results]

    fig, ax = plt.subplots(figsize=(10, 7))

    # Scatter plot
    scatter = ax.scatter(inference_times, accuracies,
                        s=[c*100 for c in channels],
                        c=channels,
                        cmap='viridis',
                        alpha=0.7,
                        edgecolors='black',
                        linewidth=2)

    # Add labels for each point
    for i, (time, acc, k) in enumerate(zip(inference_times, accuracies, channels)):
        ax.annotate(f'k={k}',
                   xy=(time, acc),
                   xytext=(5, 5),
                   textcoords='offset points',
                   fontsize=11,
                   fontweight='bold')

    ax.set_xlabel('Inference Time (ms per image)', fontsize=12, fontweight='bold')
    ax.set_ylabel('Accuracy (%)', fontsize=12, fontweight='bold')
    ax.set_title('Accuracy-Speed Tradeoff Analysis', fontsize=14, fontweight='bold')
    ax.grid(True, alpha=0.3)
    ax.set_ylim([50, 105])

    # Add colorbar
    cbar = plt.colorbar(scatter, ax=ax)
    cbar.set_label('Number of Channels (k)', fontsize=11, fontweight='bold')

    # Mark optimal point (k=3)
    optimal_idx = channels.index(3)
    ax.plot(inference_times[optimal_idx], accuracies[optimal_idx],
           'r*', markersize=25, label='Recommended (k=3)')
    ax.legend(fontsize=11, loc='lower right')

    plt.tight_layout()

    # Save figure
    output_path = Path(__file__).parent / "results" / "tradeoff_analysis.png"
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"✅ Saved: {output_path}")
    plt.close()


def generate_all_visualizations():
    """Generate all visualization plots."""
    print("\n" + "="*70)
    print("ResNet Ablation Study - Visualization Generator")
    print("="*70 + "\n")

    # Load results
    print("Loading results from JSON...")
    results = load_results()
    print(f"✅ Loaded {len(results)} experiment results\n")

    # Generate plots
    print("Generating visualizations...")
    print("-" * 70)

    plot_channels_vs_accuracy(results)
    plot_perturbation_analysis(results)
    plot_moduli_comparison(results)
    plot_consensus_impact(results)
    plot_accuracy_speed_tradeoff(results)

    print("-" * 70)
    print("\n✅ All visualizations generated successfully!")
    print(f"   Output directory: experiments/research/resnet/results/")
    print()


if __name__ == "__main__":
    generate_all_visualizations()
