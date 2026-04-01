#!/usr/bin/env python3
"""Test coach prompts against local LLM and display results."""

import json
import sys
import urllib.request
import urllib.error

BASE_URL = "https://mega4090.taild22ffc.ts.net:8081/v1"
MODEL = "Qwen3.5-35B-A3B-UD-Q3_K_XL.gguf"

SYSTEM_PROMPT = """You are Myro, a competitive programming coach in a terminal IDE. You coach **Noob123** using Socratic questioning. NEVER give solutions, formulas, or direct answers. Ask questions that guide the user to discover insights themselves.

## Problem

**Theatre Square** (difficulty: 1000)

Theatre Square in the capital city has a rectangular shape with size n x m meters. The mayor wants to cover the square with square granite flagstones of size a x a meters. Each flagstone must cover a full a x a area (no cutting). What is the minimum number of flagstones needed?

Input: Three positive integers n, m and a (1 <= n, m, a <= 10^9).
Output: Print the minimum number of flagstones needed.
Example: Input "6 6 4" -> Output "4"

## Solution Route

**Ceiling division**: Use ceiling division to compute tiles needed in each dimension, then multiply.

## Observations to Track

These are the key insights the user must discover, in order. Track progress:

- [LOCKED] `cf:1A:route:1:obs:1`: Independent dimensions
  What it means: The number of tiles along the width and height can be computed independently, then multiplied.
- [LOCKED] `cf:1A:route:1:obs:2`: Ceiling division without floats
  What it means: ceil(n/a) can be computed as (n + a - 1) / a using integer arithmetic, avoiding floating point errors.
- [LOCKED] `cf:1A:route:1:obs:3`: Use 64-bit integers
  What it means: n and a can be up to 10^9. The product ceil(n/a) * ceil(m/a) can exceed 2^31, requiring 64-bit integers.

Progress: 0/3 found | Elapsed: {elapsed}s

## State Definitions

- **"found"**: The user's code clearly demonstrates this insight (e.g., they wrote the correct formula/pattern).
- **"approaching"**: The user's code or words show partial understanding or they are heading toward this insight.
- **"moving_away"**: The user is pursuing a wrong approach (e.g., brute force when O(1) is needed, or introducing a bug). Use this when you should redirect them.
- **"uncertain"**: Not enough signal to judge. Use when code is minimal or the user just started.

## Confidence Scale

- **0.0-0.3**: Guessing / very little code to judge from.
- **0.4-0.6**: Some signal but ambiguous.
- **0.7-0.8**: Clear signal, you're fairly sure of your assessment.
- **0.9-1.0**: The code unambiguously demonstrates (or contradicts) an observation.

## Rules

1. Match the user's code against the MOST RELEVANT observation. Use the EXACT observation ID string from above (e.g., `cf:1A:route:1:obs:2`). Copy-paste it exactly. Always set matched_observation_id when you can identify which insight is relevant.
2. If the user has found one observation, guide them toward the NEXT unfound one.
3. coach_message: max 2 sentences. Must contain a question. Never reveal the answer.
4. ghost_text: almost always null. If used, NEVER include solution formulas or complete code. Only vague directional nudges.
5. next_action: a vague direction like "think about edge cases" — NEVER include formulas, code, or specific solutions.
6. If the user's approach will TLE or is fundamentally wrong, use state "moving_away" and ask about time complexity or constraints.
7. When an observation is [FOUND], do not re-match it. Focus on [LOCKED] or [APPROACHING] observations.
8. NEVER reveal the answer in ANY field. No formulas, no complete code snippets, no direct solutions.

## Output

Respond with ONLY a single JSON object. No markdown fences, no text before or after.

Fields:
- state: one of "approaching", "found", "moving_away", "uncertain"
- confidence: number between 0.0 and 1.0
- matched_observation_id: exact ID string from the observations list above, or null
- coach_message: your Socratic question (max 2 sentences, must end with ?)
- ghost_text: null (or a brief non-answer nudge in rare cases)
- ghost_format: null, "code", or "natural"
- next_action: suggested next step string, or null"""

SCENARIOS = {
    1: {
        "name": "Empty code, user stalled 60s",
        "elapsed": 60,
        "user": """Trigger: user idle for 60s



Current code:
```
n, m, a = map(int, input().split())
```

Respond with a single JSON object. Use the exact observation IDs from the system prompt.""",
    },
    2: {
        "name": "Brute force O(n*m) approach (way too slow)",
        "elapsed": 120,
        "user": """Trigger: user idle for 45s



Current code:
```
n, m, a = map(int, input().split())
count = 0
for i in range(0, n, a):
    for j in range(0, m, a):
        count += 1
print(count)
```

Respond with a single JSON object. Use the exact observation IDs from the system prompt.""",
    },
    3: {
        "name": "Float division (precision bug), test fails",
        "elapsed": 180,
        "user": """Trigger: test 1 failed: Wrong Answer

**Recent conversation**:
User: I think I need to divide n by a and m by a
Coach: Good thinking! How should you handle cases where n is not evenly divisible by a?

Current code:
```
import math
n, m, a = map(int, input().split())
rows = math.ceil(n / a)
cols = math.ceil(m / a)
print(rows * cols)
```

Respond with a single JSON object. Use the exact observation IDs from the system prompt.""",
    },
    4: {
        "name": "Correct logic, user worried about edge cases",
        "elapsed": 240,
        "user": """Trigger: user requested help

**Recent conversation**:
User: My solution works on the example but I am worried about edge cases
Coach: What are the maximum possible values from the constraints?

Current code:
```
n, m, a = map(int, input().split())
rows = (n + a - 1) // a
cols = (m + a - 1) // a
print(rows * cols)
```

Respond with a single JSON object. Use the exact observation IDs from the system prompt.""",
    },
    # -- harder problem: Xenia and Bit Operations (cf-339D, difficulty 2200) --
    5: {
        "name": "[339D] User has basic array but no segtree",
        "elapsed": 60,
        "system_override": """You are Myro, a competitive programming coach in a terminal IDE. You coach **Noob123** using Socratic questioning. NEVER give solutions, formulas, or direct answers. Ask questions that guide the user to discover insights themselves.

## Problem

**Xenia and Bit Operations** (difficulty: 2200)

You have an array of 2^n integers. Build a segment tree where odd levels compute OR and even levels compute XOR. Process q point updates and after each update print the root value.

Input: n and q (1 <= n <= 17, 1 <= q <= 10^5), then 2^n integers, then q updates.
Output: After each update, print the root value.

## Solution Route

**Segment tree with alternating operations**: Build a segment tree where the merge operation alternates between OR and XOR based on the level.

## Observations to Track

- [LOCKED] `cf:339D:route:1:obs:1`: Level determines the operation
  What it means: At the leaf level (level 0), values are stored directly. Level 1 merges pairs with OR, level 2 with XOR, alternating.
- [LOCKED] `cf:339D:route:1:obs:2`: Standard segment tree structure
  What it means: Use a 1-indexed array of size 4 * 2^n. Build bottom-up, update by changing leaf then propagating up to root.
- [LOCKED] `cf:339D:route:1:obs:3`: Propagate with correct operation per level
  What it means: When propagating up after an update, each node must use the correct operation (OR or XOR) for its level.
- [LOCKED] `cf:339D:route:1:obs:4`: O(n) per update, O(q * n) total
  What it means: Each update modifies one leaf and propagates up O(n) levels to the root.

Progress: 0/4 found | Elapsed: 60s

## State Definitions

- **"found"**: The user's code clearly demonstrates this insight.
- **"approaching"**: The user's code or words show partial understanding.
- **"moving_away"**: The user is pursuing a wrong approach. Redirect them.
- **"uncertain"**: Not enough signal to judge.

## Confidence Scale

- **0.0-0.3**: Very little code to judge from.
- **0.4-0.6**: Some signal but ambiguous.
- **0.7-0.8**: Clear signal.
- **0.9-1.0**: Unambiguous.

## Rules

1. Use the EXACT observation ID string (e.g., `cf:339D:route:1:obs:2`).
2. Guide toward the NEXT unfound observation.
3. coach_message: max 2 sentences, must contain a question, never reveal the answer.
4. ghost_text: almost always null.
5. If approach will TLE or is wrong, use "moving_away".

## Output

Respond with ONLY a single JSON object. No markdown fences, no text before or after.

Fields: state, confidence, matched_observation_id, coach_message, ghost_text, ghost_format, next_action""",
        "user": """Trigger: user idle for 60s

Current code:
```
n, q = map(int, input().split())
a = list(map(int, input().split()))
for _ in range(q):
    p, b = map(int, input().split())
    a[p-1] = b
    # how do i get the answer??
    print("???")
```

Respond with a single JSON object. Use the exact observation IDs from the system prompt.""",
    },
}

VALID_OBS_IDS = {
    1: {"cf:1A:route:1:obs:1", "cf:1A:route:1:obs:2", "cf:1A:route:1:obs:3"},
    2: {"cf:1A:route:1:obs:1", "cf:1A:route:1:obs:2", "cf:1A:route:1:obs:3"},
    3: {"cf:1A:route:1:obs:1", "cf:1A:route:1:obs:2", "cf:1A:route:1:obs:3"},
    4: {"cf:1A:route:1:obs:1", "cf:1A:route:1:obs:2", "cf:1A:route:1:obs:3"},
    5: {"cf:339D:route:1:obs:1", "cf:339D:route:1:obs:2", "cf:339D:route:1:obs:3", "cf:339D:route:1:obs:4"},
}


def call_llm(system: str, user: str) -> str:
    payload = json.dumps({
        "model": MODEL,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "max_tokens": 2048,
        "temperature": 0.7,
    }).encode()

    req = urllib.request.Request(
        f"{BASE_URL}/chat/completions",
        data=payload,
        headers={"Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            data = json.loads(resp.read())
            return data["choices"][0]["message"]["content"]
    except Exception as e:
        return f"ERROR: {e}"


def try_parse_json(raw: str) -> tuple:
    """Try to parse JSON from LLM output. Returns (parsed_dict_or_None, method, issues)."""
    issues = []
    try:
        return json.loads(raw), "direct", issues
    except json.JSONDecodeError:
        pass
    if "```json" in raw:
        try:
            start = raw.index("```json") + 7
            end = raw.index("```", start)
            parsed = json.loads(raw[start:end].strip())
            issues.append("WRAPPED_IN_FENCE")
            return parsed, "fence", issues
        except (json.JSONDecodeError, ValueError):
            pass
    if "{" in raw:
        start = raw.index("{")
        depth = 0
        for i, ch in enumerate(raw[start:]):
            if ch == "{": depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0:
                    try:
                        parsed = json.loads(raw[start:start+i+1])
                        issues.append("EMBEDDED_JSON")
                        return parsed, "embedded", issues
                    except json.JSONDecodeError:
                        break
    issues.append("INVALID_JSON")
    return None, "failed", issues


def evaluate_response(raw: str, scenario_num: int) -> dict:
    resp, method, issues = try_parse_json(raw)
    if resp is None:
        return {"raw": raw, "parsed": None, "issues": issues, "score": 0}

    score = 10
    valid_obs = VALID_OBS_IDS.get(scenario_num, set())

    # Required fields
    for f in ["state", "confidence", "matched_observation_id", "coach_message"]:
        if f not in resp:
            issues.append(f"MISSING_FIELD: {f}")
            score -= 2

    # State
    valid_states = {"approaching", "found", "moving_away", "uncertain"}
    state = resp.get("state")
    if state not in valid_states:
        issues.append(f"INVALID_STATE: '{state}'")
        score -= 2

    # Confidence
    conf = resp.get("confidence", -1)
    if not (0.0 <= conf <= 1.0):
        issues.append(f"INVALID_CONFIDENCE: {conf}")
        score -= 1

    # Obs ID
    obs_id = resp.get("matched_observation_id")
    if obs_id is not None and obs_id not in valid_obs:
        issues.append(f"INVALID_OBS_ID: '{obs_id}'")
        score -= 2

    # Socratic
    msg = resp.get("coach_message", "")
    if msg and "?" not in msg:
        issues.append("NOT_SOCRATIC: no question in coach_message")
        score -= 1

    # Ghost text leaks
    ghost = resp.get("ghost_text")
    if ghost:
        leaks = ["(n + a - 1) // a", "(n+a-1)//a", "math.ceil", "// a"]
        for pat in leaks:
            if pat in ghost:
                issues.append(f"GHOST_REVEALS_ANSWER: '{pat}'")
                score -= 3
                break

    # Scenario-specific
    if scenario_num == 1:
        if state == "found":
            issues.append("SEMANTIC: 'found' for 1-line code")
            score -= 2
        if conf > 0.5:
            issues.append(f"SEMANTIC: confidence {conf} too high for 1-line")
            score -= 1
    elif scenario_num == 2:
        if state not in ("moving_away", "approaching"):
            issues.append(f"SEMANTIC: brute force should be moving_away or approaching, got '{state}'")
            score -= 1
    elif scenario_num == 3:
        if obs_id != "cf:1A:route:1:obs:2":
            issues.append(f"SEMANTIC: should match obs:2 (float div), got {obs_id}")
            score -= 2
    elif scenario_num == 4:
        if state == "moving_away":
            issues.append("SEMANTIC: correct logic shouldn't be 'moving_away'")
            score -= 2

    return {"raw": raw, "parsed": resp, "issues": issues, "score": max(0, score)}


def main():
    scenarios_to_run = [int(x) for x in sys.argv[1:]] if len(sys.argv) > 1 else sorted(SCENARIOS.keys())

    all_results = {}
    for num in scenarios_to_run:
        sc = SCENARIOS[num]
        print(f"{'='*60}")
        print(f"SCENARIO {num}: {sc['name']}")
        print(f"{'='*60}")

        system = sc.get("system_override", SYSTEM_PROMPT).format(elapsed=sc.get("elapsed", 60))
        raw = call_llm(system, sc["user"])

        print(f"\nRAW LLM OUTPUT:")
        print(raw[:500])
        if len(raw) > 500:
            print(f"... [{len(raw)} chars total]")
        print()

        eval_result = evaluate_response(raw, num)
        all_results[num] = eval_result

        if eval_result["parsed"]:
            p = eval_result["parsed"]
            print(f"  state: {p.get('state')}")
            print(f"  confidence: {p.get('confidence')}")
            print(f"  obs_id: {p.get('matched_observation_id')}")
            print(f"  message: {p.get('coach_message')}")
            print(f"  ghost: {p.get('ghost_text')}")
        print()

        if eval_result["issues"]:
            print("ISSUES:")
            for issue in eval_result["issues"]:
                print(f"  ! {issue}")
        else:
            print("OK")
        print(f"SCORE: {eval_result['score']}/10\n")

    print(f"{'='*60}")
    print("SUMMARY")
    print(f"{'='*60}")
    total_score = 0
    max_score = 0
    for num, result in sorted(all_results.items()):
        total_score += result["score"]
        max_score += 10
        n_issues = len(result["issues"])
        status = "PASS" if n_issues == 0 else f"{n_issues} issue(s)"
        print(f"  Sc.{num}: {result['score']}/10 - {status}")
    print(f"\nTotal: {total_score}/{max_score}")


if __name__ == "__main__":
    main()
