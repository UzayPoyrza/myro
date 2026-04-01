#!/usr/bin/env python3
"""Generate analysis plots for the myro-predict evaluation report."""

import os
import numpy as np
import pandas as pd
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from sklearn.decomposition import PCA
from sklearn.preprocessing import StandardScaler

# Paths — CSVs live in the crate root (crates/myro-predict/)
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
CRATE_DIR = os.path.join(SCRIPT_DIR, "..")
OUT_DIR = os.path.join(SCRIPT_DIR, "plots")
os.makedirs(OUT_DIR, exist_ok=True)

# Style
plt.rcParams.update({
    "figure.facecolor": "white",
    "axes.facecolor": "white",
    "font.size": 11,
    "axes.titlesize": 13,
    "axes.labelsize": 11,
    "figure.dpi": 150,
})


def load_data():
    """Load all exported CSVs."""
    curve = pd.read_csv(os.path.join(CRATE_DIR, "analysis_training_curve.csv"))
    problems = pd.read_csv(os.path.join(CRATE_DIR, "analysis_problem_params.csv"))
    users = pd.read_csv(os.path.join(CRATE_DIR, "analysis_user_params.csv"))
    tag_map = pd.read_csv(os.path.join(CRATE_DIR, "analysis_tag_dim_map.csv"))
    return curve, problems, users, tag_map


def plot_training_curve(curve):
    """Plot 1: Training loss and AUC over epochs."""
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(10, 4))

    ax1.plot(curve["epoch"], curve["loss"], "o-", color="#2563eb", markersize=3, linewidth=1.5)
    ax1.set_xlabel("Epoch")
    ax1.set_ylabel("Log-loss")
    ax1.set_title("Training Loss")
    ax1.grid(True, alpha=0.3)

    ax2.plot(curve["epoch"], curve["train_auc"], "o-", color="#dc2626", markersize=3, linewidth=1.5)
    ax2.set_xlabel("Epoch")
    ax2.set_ylabel("AUC-ROC")
    ax2.set_title("Training AUC")
    ax2.set_ylim(0.90, 1.0)
    ax2.grid(True, alpha=0.3)

    fig.tight_layout()
    fig.savefig(os.path.join(OUT_DIR, "training_curve.png"), bbox_inches="tight")
    plt.close()
    print("  Saved training_curve.png")


def plot_difficulty_vs_rating(problems):
    """Plot 2: Learned difficulty bias vs CF problem rating."""
    mask = problems["rating"].notna()
    df = problems[mask].copy()

    fig, ax = plt.subplots(figsize=(7, 5))
    ax.scatter(df["rating"], df["difficulty"], alpha=0.3, s=8, color="#6366f1")

    # Trend line
    z = np.polyfit(df["rating"], df["difficulty"], 1)
    x_line = np.linspace(df["rating"].min(), df["rating"].max(), 100)
    ax.plot(x_line, np.polyval(z, x_line), "r-", linewidth=2, label=f"Linear fit (slope={z[0]:.4f})")

    ax.set_xlabel("CF Problem Rating")
    ax.set_ylabel("Learned Difficulty Bias (d_p)")
    ax.set_title("Learned Difficulty vs Codeforces Rating")
    ax.legend()
    ax.grid(True, alpha=0.3)
    corr = df["rating"].corr(df["difficulty"])
    ax.text(0.05, 0.95, f"r = {corr:.3f}", transform=ax.transAxes,
            fontsize=11, verticalalignment="top",
            bbox=dict(boxstyle="round", facecolor="wheat", alpha=0.5))

    fig.tight_layout()
    fig.savefig(os.path.join(OUT_DIR, "difficulty_vs_rating.png"), bbox_inches="tight")
    plt.close()
    print("  Saved difficulty_vs_rating.png")


def plot_problem_pca(problems, tag_map):
    """Plot 3: PCA of problem latent vectors, colored by primary tag."""
    alpha_cols = [c for c in problems.columns if c.startswith("alpha_")]
    X = problems[alpha_cols].values

    scaler = StandardScaler()
    X_scaled = scaler.fit_transform(X)

    pca = PCA(n_components=2)
    X_pca = pca.fit_transform(X_scaled)

    # Get primary tag for each problem (first tag)
    problems = problems.copy()
    problems["primary_tag"] = problems["tags"].fillna("").apply(
        lambda t: t.split(";")[0] if t else "unknown"
    )

    # Top 8 tags by frequency
    top_tags = problems["primary_tag"].value_counts().head(8).index.tolist()

    fig, ax = plt.subplots(figsize=(9, 7))

    colors = plt.cm.tab10(np.linspace(0, 1, 10))
    for i, tag in enumerate(top_tags):
        mask = problems["primary_tag"] == tag
        ax.scatter(X_pca[mask, 0], X_pca[mask, 1],
                   alpha=0.5, s=15, color=colors[i], label=tag)

    # Others
    other_mask = ~problems["primary_tag"].isin(top_tags)
    ax.scatter(X_pca[other_mask, 0], X_pca[other_mask, 1],
               alpha=0.15, s=8, color="gray", label="other")

    ax.set_xlabel(f"PC1 ({pca.explained_variance_ratio_[0]*100:.1f}% var)")
    ax.set_ylabel(f"PC2 ({pca.explained_variance_ratio_[1]*100:.1f}% var)")
    ax.set_title("Problem Latent Space (PCA)")
    ax.legend(fontsize=8, markerscale=2, loc="best")
    ax.grid(True, alpha=0.3)

    fig.tight_layout()
    fig.savefig(os.path.join(OUT_DIR, "problem_pca.png"), bbox_inches="tight")
    plt.close()
    print("  Saved problem_pca.png")

    return pca


def plot_tag_dimension_heatmap(problems, tag_map):
    """Plot 4: Average activation of each latent dimension per tag."""
    alpha_cols = [c for c in problems.columns if c.startswith("alpha_")]
    problems = problems.copy()

    # Explode tags
    rows = []
    for _, row in problems.iterrows():
        tags_str = row["tags"] if pd.notna(row["tags"]) else ""
        tags = [t.strip() for t in tags_str.split(";") if t.strip()]
        for tag in tags:
            row_data = {"tag": tag}
            for col in alpha_cols:
                row_data[col] = row[col]
            rows.append(row_data)

    tag_df = pd.DataFrame(rows)
    if tag_df.empty:
        print("  Skipping tag heatmap (no tag data)")
        return

    # Top 15 tags
    top_tags = tag_df["tag"].value_counts().head(15).index.tolist()
    tag_df = tag_df[tag_df["tag"].isin(top_tags)]

    # Mean alpha per tag
    means = tag_df.groupby("tag")[alpha_cols].mean()
    means = means.loc[top_tags]

    fig, ax = plt.subplots(figsize=(12, 6))
    im = ax.imshow(means.values, aspect="auto", cmap="RdBu_r", vmin=-0.3, vmax=0.3)

    ax.set_yticks(range(len(top_tags)))
    ax.set_yticklabels(top_tags, fontsize=9)
    ax.set_xticks(range(len(alpha_cols)))
    ax.set_xticklabels([f"d{i}" for i in range(len(alpha_cols))], fontsize=8, rotation=45)
    ax.set_xlabel("Latent Dimension")
    ax.set_ylabel("CF Tag")
    ax.set_title("Mean Latent Activation per Tag")

    # Annotate tag_dim_map assignments
    for _, row in tag_map.iterrows():
        tag = row["tag"]
        dim = row["dimension"]
        if tag in top_tags:
            y_idx = top_tags.index(tag)
            ax.add_patch(plt.Rectangle((dim - 0.5, y_idx - 0.5), 1, 1,
                                        fill=False, edgecolor="black", linewidth=2))

    plt.colorbar(im, ax=ax, label="Mean alpha", shrink=0.8)
    fig.tight_layout()
    fig.savefig(os.path.join(OUT_DIR, "tag_dimension_heatmap.png"), bbox_inches="tight")
    plt.close()
    print("  Saved tag_dimension_heatmap.png")


def plot_tag_skill_correspondence(problems, tag_map):
    """Plot 5: For each tag-initialized dimension, compare mean activation of
    matching vs non-matching problems."""
    alpha_cols = [c for c in problems.columns if c.startswith("alpha_")]
    problems = problems.copy()

    results = []
    for _, row in tag_map.iterrows():
        tag = row["tag"]
        dim = int(row["dimension"])
        col = f"alpha_{dim}"

        # Problems with this tag
        has_tag = problems["tags"].fillna("").str.contains(tag, regex=False)
        mean_with = problems.loc[has_tag, col].mean()
        mean_without = problems.loc[~has_tag, col].mean()
        n_with = has_tag.sum()

        results.append({
            "tag": tag,
            "dimension": dim,
            "mean_with_tag": mean_with,
            "mean_without_tag": mean_without,
            "n_problems": n_with,
        })

    res_df = pd.DataFrame(results).sort_values("n_problems", ascending=True)

    fig, ax = plt.subplots(figsize=(8, 7))
    y_pos = range(len(res_df))

    ax.barh(y_pos, res_df["mean_with_tag"], height=0.4, align="edge",
            color="#2563eb", alpha=0.8, label="Has tag")
    ax.barh([y + 0.4 for y in y_pos], res_df["mean_without_tag"], height=0.4,
            align="edge", color="#dc2626", alpha=0.5, label="Without tag")

    ax.set_yticks([y + 0.4 for y in y_pos])
    ax.set_yticklabels([f"{row.tag} (d{row.dimension}, n={row.n_problems})"
                        for row in res_df.itertuples()], fontsize=8)
    ax.set_xlabel("Mean alpha activation on assigned dimension")
    ax.set_title("Tag-Informed Initialization: Learned Skill Correspondence")
    ax.legend()
    ax.grid(True, alpha=0.3, axis="x")
    ax.axvline(x=0, color="black", linewidth=0.5)

    fig.tight_layout()
    fig.savefig(os.path.join(OUT_DIR, "tag_skill_correspondence.png"), bbox_inches="tight")
    plt.close()
    print("  Saved tag_skill_correspondence.png")


def plot_user_bias_distribution(users):
    """Plot 6: Distribution of user ability bias."""
    fig, ax = plt.subplots(figsize=(7, 4))
    ax.hist(users["bias"], bins=80, color="#6366f1", alpha=0.7, edgecolor="white", linewidth=0.5)
    ax.set_xlabel("User Ability Bias (b_u)")
    ax.set_ylabel("Count")
    ax.set_title("Distribution of Learned User Ability")
    ax.axvline(users["bias"].mean(), color="red", linestyle="--",
               label=f"Mean = {users['bias'].mean():.2f}")
    ax.axvline(users["bias"].median(), color="orange", linestyle="--",
               label=f"Median = {users['bias'].median():.2f}")
    ax.legend()
    ax.grid(True, alpha=0.3)

    fig.tight_layout()
    fig.savefig(os.path.join(OUT_DIR, "user_bias_distribution.png"), bbox_inches="tight")
    plt.close()
    print("  Saved user_bias_distribution.png")


def plot_eval_comparison():
    """Plot 7: Bar chart of evaluation results."""
    methods = ["Random", "Solve Rate", "Elo", "LogReg\n(rating+tags)", "MF (ours)"]
    auc_values = [0.4991, 0.9166, 0.9183, 0.9210, 0.9478]
    logloss_values = [0.6931, 0.3438, 0.4079, 0.3371, 0.2825]
    colors = ["#94a3b8", "#60a5fa", "#34d399", "#fbbf24", "#ef4444"]

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(11, 4.5))

    bars1 = ax1.bar(methods, auc_values, color=colors, edgecolor="white", linewidth=1)
    ax1.set_ylabel("AUC-ROC")
    ax1.set_title("Prediction Quality (AUC, higher is better)")
    ax1.set_ylim(0.45, 1.0)
    ax1.grid(True, alpha=0.3, axis="y")
    for bar, val in zip(bars1, auc_values):
        ax1.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 0.005,
                 f"{val:.4f}", ha="center", va="bottom", fontsize=9)

    bars2 = ax2.bar(methods, logloss_values, color=colors, edgecolor="white", linewidth=1)
    ax2.set_ylabel("Log-loss")
    ax2.set_title("Calibration (Log-loss, lower is better)")
    ax2.set_ylim(0, 0.8)
    ax2.grid(True, alpha=0.3, axis="y")
    for bar, val in zip(bars2, logloss_values):
        ax2.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 0.008,
                 f"{val:.4f}", ha="center", va="bottom", fontsize=9)

    fig.tight_layout()
    fig.savefig(os.path.join(OUT_DIR, "eval_comparison.png"), bbox_inches="tight")
    plt.close()
    print("  Saved eval_comparison.png")


def plot_per_band_auc():
    """Plot 8: Per-rating-band AUC breakdown for MF model."""
    bands = ["800-1200", "1200-1600", "1600-2000", "2000-2400", "2400-3500"]
    aucs = [0.8046, 0.8375, 0.8931, 0.9407, 0.9510]
    loglosses = [0.3091, 0.4872, 0.3449, 0.1481, 0.0708]
    counts = [63474, 84897, 102358, 85827, 65780]

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(11, 4.5))

    color_gradient = plt.cm.viridis(np.linspace(0.2, 0.8, len(bands)))

    bars = ax1.bar(bands, aucs, color=color_gradient, edgecolor="white")
    ax1.set_xlabel("Problem Rating Band")
    ax1.set_ylabel("AUC-ROC")
    ax1.set_title("MF Model AUC by Problem Difficulty")
    ax1.set_ylim(0.75, 1.0)
    ax1.grid(True, alpha=0.3, axis="y")
    for bar, val in zip(bars, aucs):
        ax1.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 0.003,
                 f"{val:.3f}", ha="center", va="bottom", fontsize=9)

    bars2 = ax2.bar(bands, loglosses, color=color_gradient, edgecolor="white")
    ax2.set_xlabel("Problem Rating Band")
    ax2.set_ylabel("Log-loss")
    ax2.set_title("MF Model Calibration by Problem Difficulty")
    ax2.set_ylim(0, 0.6)
    ax2.grid(True, alpha=0.3, axis="y")
    for bar, val in zip(bars2, loglosses):
        ax2.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 0.005,
                 f"{val:.3f}", ha="center", va="bottom", fontsize=9)

    fig.tight_layout()
    fig.savefig(os.path.join(OUT_DIR, "per_band_breakdown.png"), bbox_inches="tight")
    plt.close()
    print("  Saved per_band_breakdown.png")


def main():
    print("Loading data...")
    curve, problems, users, tag_map = load_data()
    print(f"  Training curve: {len(curve)} epochs")
    print(f"  Problems: {len(problems)} rows")
    print(f"  Users: {len(users)} rows")
    print(f"  Tag-dim map: {len(tag_map)} tags")

    print("\nGenerating plots...")
    plot_training_curve(curve)
    plot_difficulty_vs_rating(problems)
    plot_problem_pca(problems, tag_map)
    plot_tag_dimension_heatmap(problems, tag_map)
    plot_tag_skill_correspondence(problems, tag_map)
    plot_user_bias_distribution(users)
    plot_eval_comparison()
    plot_per_band_auc()

    print(f"\nAll plots saved to {OUT_DIR}/")


if __name__ == "__main__":
    main()
