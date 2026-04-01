#!/usr/bin/env python3
"""Generate divergence analysis charts from export_predictions CSV.

Usage: python3 divergence_chart.py <predictions.csv> [output_prefix]

Produces:
  - {prefix}_divergence_scatter.png  — MF vs Elo scatter with divergence coloring
  - {prefix}_divergence_by_rating.png — Divergence by problem rating band
  - {prefix}_top_divergent.csv — Top 50 most divergent problems (MF > Elo and MF < Elo)
"""

import csv
import sys
from pathlib import Path

try:
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    import numpy as np
except ImportError:
    print("ERROR: matplotlib and numpy required. Install with: pip install matplotlib numpy")
    sys.exit(1)


def load_predictions(csv_path):
    rows = []
    with open(csv_path) as f:
        reader = csv.DictReader(f)
        for row in reader:
            try:
                r = {
                    "key": row["problem_key"],
                    "name": row["problem_name"],
                    "link": row["problem_link"],
                    "rating": int(row["rating"]) if row["rating"] else None,
                    "tags": row["tags"],
                    "p_mf": float(row["p_solve_mf"]),
                    "p_elo": float(row["p_solve_elo"]),
                    "divergence": float(row["divergence"]),
                    "solved": row["solved"].lower() == "true",
                }
                rows.append(r)
            except (ValueError, KeyError):
                continue
    return rows


def scatter_plot(rows, output_path):
    """MF vs Elo scatter colored by divergence magnitude."""
    rated = [r for r in rows if r["rating"] is not None]
    if not rated:
        print("No rated problems for scatter plot")
        return

    p_mf = np.array([r["p_mf"] for r in rated])
    p_elo = np.array([r["p_elo"] for r in rated])
    div = np.array([r["divergence"] for r in rated])
    solved = np.array([r["solved"] for r in rated])

    fig, ax = plt.subplots(figsize=(10, 8))

    # Color by divergence
    sc = ax.scatter(
        p_elo[~solved], p_mf[~solved],
        c=div[~solved], cmap="RdYlBu_r", alpha=0.4, s=15,
        vmin=-0.5, vmax=0.5, label="Unsolved",
    )
    ax.scatter(
        p_elo[solved], p_mf[solved],
        c=div[solved], cmap="RdYlBu_r", alpha=0.8, s=30,
        vmin=-0.5, vmax=0.5, marker="*", label="Solved",
    )

    # Diagonal
    ax.plot([0, 1], [0, 1], "k--", alpha=0.3, linewidth=1)
    ax.set_xlabel("P(solve) — Elo baseline", fontsize=12)
    ax.set_ylabel("P(solve) — MF model", fontsize=12)
    ax.set_title("MF vs Elo Predictions — Divergence Analysis", fontsize=14)
    ax.legend(loc="upper left")

    cbar = plt.colorbar(sc, ax=ax)
    cbar.set_label("Divergence (MF − Elo)", fontsize=11)

    ax.set_xlim(-0.02, 1.02)
    ax.set_ylim(-0.02, 1.02)
    ax.set_aspect("equal")
    fig.tight_layout()
    fig.savefig(output_path, dpi=150)
    plt.close(fig)
    print(f"  Scatter plot: {output_path}")


def rating_band_plot(rows, output_path):
    """Divergence distribution by problem rating band."""
    rated = [r for r in rows if r["rating"] is not None]
    if not rated:
        print("No rated problems for rating band plot")
        return

    # Group by 200-rating bands
    bands = {}
    for r in rated:
        band = (r["rating"] // 200) * 200
        bands.setdefault(band, []).append(r["divergence"])

    sorted_bands = sorted(bands.keys())
    labels = [f"{b}-{b+199}" for b in sorted_bands]
    means = [np.mean(bands[b]) for b in sorted_bands]
    stds = [np.std(bands[b]) for b in sorted_bands]
    counts = [len(bands[b]) for b in sorted_bands]

    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(12, 8), height_ratios=[3, 1])

    colors = ["#e74c3c" if m > 0.05 else "#3498db" if m < -0.05 else "#95a5a6" for m in means]
    bars = ax1.bar(range(len(labels)), means, yerr=stds, capsize=3, color=colors, alpha=0.7)
    ax1.axhline(0, color="black", linewidth=0.5)
    ax1.set_ylabel("Mean Divergence (MF − Elo)", fontsize=11)
    ax1.set_title("Where MF Model Disagrees with Elo — by Rating Band", fontsize=13)
    ax1.set_xticks(range(len(labels)))
    ax1.set_xticklabels(labels, rotation=45, ha="right", fontsize=9)

    # Annotate: positive = MF thinks easier, negative = MF thinks harder
    ax1.text(0.02, 0.95, "↑ MF thinks easier than Elo", transform=ax1.transAxes,
             fontsize=9, color="#e74c3c", va="top")
    ax1.text(0.02, 0.05, "↓ MF thinks harder than Elo", transform=ax1.transAxes,
             fontsize=9, color="#3498db")

    # Count subplot
    ax2.bar(range(len(labels)), counts, color="#7f8c8d", alpha=0.5)
    ax2.set_ylabel("# Problems", fontsize=10)
    ax2.set_xticks(range(len(labels)))
    ax2.set_xticklabels(labels, rotation=45, ha="right", fontsize=9)

    fig.tight_layout()
    fig.savefig(output_path, dpi=150)
    plt.close(fig)
    print(f"  Rating band plot: {output_path}")


def top_divergent_csv(rows, output_path, n=50):
    """Export top N most divergent problems in each direction."""
    rated = [r for r in rows if r["rating"] is not None]
    if not rated:
        print("No rated problems for divergence CSV")
        return

    # Sort by absolute divergence, take top N from each direction
    mf_easier = sorted([r for r in rated if r["divergence"] > 0],
                        key=lambda r: -r["divergence"])[:n]
    mf_harder = sorted([r for r in rated if r["divergence"] < 0],
                        key=lambda r: r["divergence"])[:n]

    with open(output_path, "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["direction", "problem_key", "problem_name", "link", "rating",
                          "tags", "p_solve_mf", "p_solve_elo", "divergence", "solved"])
        for r in mf_easier:
            writer.writerow(["MF_easier", r["key"], r["name"], r["link"], r["rating"],
                              r["tags"], f'{r["p_mf"]:.4f}', f'{r["p_elo"]:.4f}',
                              f'{r["divergence"]:.4f}', r["solved"]])
        for r in mf_harder:
            writer.writerow(["MF_harder", r["key"], r["name"], r["link"], r["rating"],
                              r["tags"], f'{r["p_mf"]:.4f}', f'{r["p_elo"]:.4f}',
                              f'{r["divergence"]:.4f}', r["solved"]])

    print(f"  Top divergent CSV: {output_path} ({len(mf_easier)} easier + {len(mf_harder)} harder)")


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    csv_path = sys.argv[1]
    prefix = sys.argv[2] if len(sys.argv) > 2 else Path(csv_path).stem

    print(f"Loading predictions from {csv_path}...")
    rows = load_predictions(csv_path)
    print(f"Loaded {len(rows)} problems ({sum(1 for r in rows if r['rating'])} rated)")

    print("\nGenerating charts:")
    scatter_plot(rows, f"{prefix}_divergence_scatter.png")
    rating_band_plot(rows, f"{prefix}_divergence_by_rating.png")
    top_divergent_csv(rows, f"{prefix}_top_divergent.csv")

    # Print summary stats
    rated = [r for r in rows if r["rating"] is not None]
    if rated:
        divs = [r["divergence"] for r in rated]
        print(f"\nDivergence stats (rated problems only):")
        print(f"  Mean:   {np.mean(divs):+.4f}")
        print(f"  Std:    {np.std(divs):.4f}")
        print(f"  Min:    {np.min(divs):+.4f}")
        print(f"  Max:    {np.max(divs):+.4f}")
        print(f"  |div|>0.2: {sum(1 for d in divs if abs(d) > 0.2)} problems")


if __name__ == "__main__":
    main()
