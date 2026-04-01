#!/usr/bin/env python3
"""Fetch and select 100 Codeforces problems (rated 1200-2200) with editorials.

Pipeline:
  CF API → Filter → Select 100 (balanced buckets) → Scrape HTML → Parse → Save raw JSON

Dependencies: pip install requests beautifulsoup4
"""

import json
import os
import re
import sys
import time
from collections import defaultdict
from pathlib import Path

import requests
from bs4 import BeautifulSoup

REPO_ROOT = Path(__file__).resolve().parent.parent
PROBLEM_SET_DIR = REPO_ROOT / "test-problem-set"
RAW_DIR = REPO_ROOT / "raw-problems"

SESSION = requests.Session()
SESSION.headers.update({
    "User-Agent": "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0",
    "Accept-Language": "en-US,en;q=0.9",
})

# Difficulty buckets: center → (low, high)
BUCKETS = {
    1200: (1100, 1300),
    1400: (1300, 1500),
    1600: (1500, 1700),
    1800: (1700, 1900),
    2000: (1900, 2100),
    2200: (2100, 2300),
}
TARGET_PER_BUCKET = 17  # 17 * 6 = 102, we'll trim to 100

EXCLUDED_TAGS = {"interactive", "special", "*special"}

# Preferred topic diversity ordering
PREFERRED_TAGS = [
    "dp", "greedy", "graphs", "math", "strings", "data structures",
    "binary search", "number theory", "trees", "geometry",
    "combinatorics", "constructive algorithms", "sortings",
    "two pointers", "dfs and similar", "bfs", "shortest paths",
    "hashing", "implementation", "brute force",
]

REQUEST_DELAY = 2.0  # seconds between CF requests


def fetch_with_delay(url: str, **kwargs) -> requests.Response:
    """Fetch URL with rate limiting and retries."""
    for attempt in range(3):
        try:
            time.sleep(REQUEST_DELAY)
            resp = SESSION.get(url, timeout=30, **kwargs)
            if resp.status_code == 503:
                print(f"  503 on {url}, retrying in 5s...")
                time.sleep(5)
                continue
            resp.raise_for_status()
            return resp
        except requests.RequestException as e:
            if attempt == 2:
                raise
            print(f"  Retry {attempt+1} for {url}: {e}")
            time.sleep(3)
    raise RuntimeError(f"Failed to fetch {url}")


def get_existing_ids() -> set[str]:
    """Get contest_id + index combos that already exist in test-problem-set/."""
    ids = set()
    for f in PROBLEM_SET_DIR.glob("cf-*.json"):
        # cf-1A.json → "1A"
        name = f.stem.removeprefix("cf-")
        ids.add(name)
    return ids


def fetch_problem_list() -> list[dict]:
    """Fetch all problems from CF API."""
    print("Fetching problem list from CF API...")
    resp = SESSION.get(
        "https://codeforces.com/api/problemset.problems",
        timeout=30,
    )
    resp.raise_for_status()
    data = resp.json()
    if data["status"] != "OK":
        raise RuntimeError(f"CF API error: {data}")

    problems = data["result"]["problems"]
    stats = {
        (s["contestId"], s["index"]): s.get("solvedCount", 0)
        for s in data["result"]["problemStatistics"]
    }

    for p in problems:
        key = (p.get("contestId"), p.get("index"))
        p["solvedCount"] = stats.get(key, 0)

    print(f"  Got {len(problems)} problems total")
    return problems


def select_problems(problems: list[dict], existing: set[str]) -> list[dict]:
    """Select 100 problems with balanced difficulty and topic diversity."""
    # Filter
    candidates = []
    for p in problems:
        cid = p.get("contestId")
        idx = p.get("index", "")
        rating = p.get("rating")
        tags = set(p.get("tags", []))

        if not cid or not rating:
            continue
        if f"{cid}{idx}" in existing:
            continue
        if tags & EXCLUDED_TAGS:
            continue
        if rating < 1100 or rating > 2300:
            continue

        candidates.append(p)

    print(f"  {len(candidates)} candidates after filtering")

    # Bucket candidates
    bucketed = defaultdict(list)
    for p in candidates:
        rating = p["rating"]
        for center, (lo, hi) in BUCKETS.items():
            if lo <= rating < hi:
                bucketed[center].append(p)
                break

    # Within each bucket, sort for topic diversity then popularity
    selected = []
    for center in sorted(BUCKETS.keys()):
        pool = bucketed[center]
        if not pool:
            print(f"  WARNING: No candidates for bucket {center}")
            continue

        # Score each problem: prefer diverse tags, then high solvedCount
        tag_counts = defaultdict(int)  # track how many selected have each tag
        bucket_selected = []

        # Sort pool by solvedCount desc as tiebreaker
        pool.sort(key=lambda p: p["solvedCount"], reverse=True)

        for _ in range(min(TARGET_PER_BUCKET, len(pool))):
            best = None
            best_score = -1

            for p in pool:
                if p in bucket_selected:
                    continue
                tags = p.get("tags", [])
                # Score: prefer tags we haven't seen much
                novelty = sum(1 for t in tags if tag_counts[t] < 2)
                # Bonus for preferred tags
                pref_bonus = sum(0.5 for t in tags if t in PREFERRED_TAGS)
                score = novelty + pref_bonus + p["solvedCount"] / 1_000_000
                if score > best_score:
                    best_score = score
                    best = p

            if best:
                bucket_selected.append(best)
                for t in best.get("tags", []):
                    tag_counts[t] += 1

        selected.extend(bucket_selected)
        print(f"  Bucket {center}: selected {len(bucket_selected)} problems")

    # Trim to exactly 100
    selected = selected[:100]
    print(f"  Total selected: {len(selected)}")
    return selected


def scrape_problem(contest_id: int, index: str) -> dict | None:
    """Scrape a problem page and parse it."""
    url = f"https://codeforces.com/problemset/problem/{contest_id}/{index}"
    try:
        resp = fetch_with_delay(url)
    except Exception as e:
        print(f"  FAILED to fetch {contest_id}{index}: {e}")
        return None

    soup = BeautifulSoup(resp.text, "html.parser")
    ps = soup.select_one(".problem-statement")
    if not ps:
        print(f"  No .problem-statement found for {contest_id}{index}")
        return None

    # Title
    title_el = ps.select_one(".header .title")
    title = title_el.get_text(strip=True) if title_el else ""
    # Remove leading problem letter like "A. " or "B. "
    title = re.sub(r"^[A-Z]\d*\.\s*", "", title)

    # Time/memory limits
    time_el = ps.select_one(".header .time-limit")
    time_limit = ""
    if time_el:
        time_limit = time_el.get_text(strip=True).replace("time limit per test", "").strip()

    mem_el = ps.select_one(".header .memory-limit")
    memory_limit = ""
    if mem_el:
        memory_limit = mem_el.get_text(strip=True).replace("memory limit per test", "").strip()

    # Description: divs in .problem-statement that aren't known sections
    known_classes = {"header", "input-specification", "output-specification", "sample-tests", "note"}
    desc_parts = []
    for child in ps.children:
        if hasattr(child, "get") and child.get("class"):
            classes = set(child.get("class", []))
            if classes & known_classes:
                continue
        if hasattr(child, "get_text"):
            text = child.get_text(separator=" ", strip=True)
            if text:
                desc_parts.append(text)
    description = "\n\n".join(desc_parts)

    # Input spec
    input_el = ps.select_one(".input-specification")
    input_spec = ""
    if input_el:
        input_spec = input_el.get_text(separator=" ", strip=True)
        input_spec = re.sub(r"^Input\s*", "", input_spec).strip()

    # Output spec
    output_el = ps.select_one(".output-specification")
    output_spec = ""
    if output_el:
        output_spec = output_el.get_text(separator=" ", strip=True)
        output_spec = re.sub(r"^Output\s*", "", output_spec).strip()

    # Examples
    examples = []
    input_pres = ps.select(".sample-tests .input pre")
    output_pres = ps.select(".sample-tests .output pre")
    for inp, outp in zip(input_pres, output_pres):
        # Handle <br> tags in pre blocks
        for br in inp.find_all("br"):
            br.replace_with("\n")
        for br in outp.find_all("br"):
            br.replace_with("\n")
        examples.append({
            "input": inp.get_text(strip=True),
            "output": outp.get_text(strip=True),
        })

    # Note
    note_el = ps.select_one(".note")
    note = None
    if note_el:
        note_text = note_el.get_text(separator=" ", strip=True)
        note_text = re.sub(r"^Note\s*", "", note_text).strip()
        if note_text:
            note = note_text

    return {
        "title": title,
        "time_limit": time_limit,
        "memory_limit": memory_limit,
        "description": description,
        "input_spec": input_spec,
        "output_spec": output_spec,
        "examples": examples,
        "note": note,
    }


def fetch_editorial(contest_id: int) -> dict[str, str]:
    """Fetch editorial for a contest. Returns {index: editorial_text}."""
    editorials = {}

    # Try contest page for tutorial/editorial link
    contest_url = f"https://codeforces.com/contest/{contest_id}"
    try:
        resp = fetch_with_delay(contest_url)
    except Exception as e:
        print(f"  Could not fetch contest page for {contest_id}: {e}")
        return editorials

    soup = BeautifulSoup(resp.text, "html.parser")

    # Look for editorial link in sidebar or announcements
    editorial_url = None
    for a in soup.select("a"):
        href = a.get("href", "")
        text = a.get_text(strip=True).lower()
        if any(kw in text for kw in ["editorial", "tutorial", "разбор", "analysis"]):
            if "/blog/entry/" in href:
                if href.startswith("/"):
                    editorial_url = "https://codeforces.com" + href
                else:
                    editorial_url = href
                break

    if not editorial_url:
        # Try common pattern: search CF blog
        print(f"  No editorial link found for contest {contest_id}")
        return editorials

    # Fetch the editorial blog post
    try:
        resp = fetch_with_delay(editorial_url)
    except Exception as e:
        print(f"  Could not fetch editorial blog for {contest_id}: {e}")
        return editorials

    soup = BeautifulSoup(resp.text, "html.parser")
    content = soup.select_one(".ttypography")
    if not content:
        content = soup.select_one("#pageContent")
    if not content:
        return editorials

    # Extract full editorial text
    full_text = content.get_text(separator="\n", strip=True)

    # Try to split by problem (look for "Problem A", "A.", etc.)
    # We'll store the whole editorial and let subagents extract what they need
    editorials["_full"] = full_text

    # Also try to extract per-problem sections
    # Common patterns: "Problem A", "A. Title", "Problem A —", div with problem headers
    sections = re.split(
        r'\n(?=(?:Problem\s+[A-Z]\d?[\s.—:-]|[A-Z]\d?\s*[.—:-]\s+[A-Z]))',
        full_text,
    )
    if len(sections) > 1:
        for section in sections:
            match = re.match(r'(?:Problem\s+)?([A-Z]\d?)\s*[.—:-]', section)
            if match:
                idx = match.group(1)
                editorials[idx] = section.strip()

    return editorials


def main():
    RAW_DIR.mkdir(exist_ok=True)

    # Check what we already have in raw-problems (for resumability)
    already_fetched = set()
    for f in RAW_DIR.glob("cf-*.json"):
        already_fetched.add(f.stem.removeprefix("cf-"))

    existing = get_existing_ids()
    print(f"Existing problems in test-problem-set: {len(existing)}")
    print(f"Already fetched raw problems: {len(already_fetched)}")

    problems = fetch_problem_list()
    selected = select_problems(problems, existing)

    if not selected:
        print("No problems to fetch!")
        return

    # Group by contest for editorial fetching efficiency
    by_contest = defaultdict(list)
    for p in selected:
        by_contest[p["contestId"]].append(p)

    print(f"\nProblems span {len(by_contest)} contests")

    success = 0
    skipped = 0
    failed = 0
    editorial_cache = {}

    for i, p in enumerate(selected):
        cid = p["contestId"]
        idx = p["index"]
        pid = f"{cid}{idx}"

        if pid in already_fetched:
            print(f"[{i+1}/{len(selected)}] {pid} — already fetched, skipping")
            skipped += 1
            continue

        print(f"[{i+1}/{len(selected)}] Scraping {pid} ({p.get('name', '')})...")

        parsed = scrape_problem(cid, idx)
        if not parsed:
            failed += 1
            continue

        # Fetch editorial for this contest if not cached
        if cid not in editorial_cache:
            print(f"  Fetching editorial for contest {cid}...")
            editorial_cache[cid] = fetch_editorial(cid)

        editorial_text = editorial_cache[cid].get(idx) or editorial_cache[cid].get("_full", "")
        # Truncate editorial if extremely long
        if len(editorial_text) > 8000:
            editorial_text = editorial_text[:8000] + "\n[truncated]"

        raw = {
            "contest_id": cid,
            "index": idx,
            "title": parsed["title"],
            "difficulty": p.get("rating", 0),
            "tags": p.get("tags", []),
            "time_limit": parsed["time_limit"] or None,
            "memory_limit": parsed["memory_limit"] or None,
            "description": parsed["description"],
            "input_spec": parsed["input_spec"],
            "output_spec": parsed["output_spec"],
            "examples": parsed["examples"],
            "note": parsed.get("note"),
            "editorial": editorial_text or None,
        }

        out_path = RAW_DIR / f"cf-{pid}.json"
        with open(out_path, "w") as f:
            json.dump(raw, f, indent=2, ensure_ascii=False)

        success += 1
        print(f"  Saved → {out_path.name}")

    print(f"\nDone: {success} fetched, {skipped} skipped, {failed} failed")

    # Print distribution summary
    print("\nDifficulty distribution:")
    dist = defaultdict(int)
    for f in RAW_DIR.glob("cf-*.json"):
        with open(f) as fh:
            d = json.load(fh)
            rating = d.get("difficulty", 0)
            for center, (lo, hi) in BUCKETS.items():
                if lo <= rating < hi:
                    dist[center] += 1
                    break
    for center in sorted(dist):
        print(f"  {center}: {dist[center]}")


if __name__ == "__main__":
    main()
