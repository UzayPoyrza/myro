/**
 * Seed data: 10 problems with 1–2 routes each, 4–7 observations per route.
 */

export interface SeedObservation {
  order: number;
  title: string;
  description: string;
  hints: [string, string, string]; // [nudge, pointed_question, partial_formalization]
}

export interface SeedRoute {
  name: string;
  description: string;
  order: number;
  observations: SeedObservation[];
}

export interface SeedProblem {
  title: string;
  difficulty: number;
  topic: string;
  description: string;
  inputSpec: string;
  outputSpec: string;
  examples: { input: string; output: string }[];
  routes: SeedRoute[];
}

export const SEED_PROBLEMS: SeedProblem[] = [
  {
    title: "Two Sum",
    difficulty: 1000,
    topic: "arrays, hashing",
    description:
      "Given an array of n integers and a target sum, find two numbers that add up to the target. Return their indices.",
    inputSpec:
      "First line: n and target. Second line: n space-separated integers.",
    outputSpec: "Two 0-indexed indices of the numbers that sum to target.",
    examples: [
      { input: "4 9\n2 7 11 15", output: "0 1" },
      { input: "3 6\n3 2 4", output: "1 2" },
    ],
    routes: [
      {
        name: "Hash Map Approach",
        description:
          "Use a hash map to store seen values and check for complements in O(n) time.",
        order: 1,
        observations: [
          {
            order: 1,
            title: "Complement relationship",
            description:
              "For each number x, we need target - x to exist somewhere in the array.",
            hints: [
              "If you know one number, what must the other number be?",
              "For a number x in the array, what value would you need to find to reach the target?",
              "Each number x has a complement (target - x). The problem reduces to finding if any complement exists in the array.",
            ],
          },
          {
            order: 2,
            title: "Lookup structure",
            description:
              "A hash map enables O(1) lookup to check if a complement has been seen.",
            hints: [
              "How can you quickly check if a specific value exists in the array?",
              "What data structure gives you O(1) lookup? How would you populate it?",
              "Store each number in a hash map as you iterate. For each new number, check if its complement is already in the map.",
            ],
          },
          {
            order: 3,
            title: "Single-pass sufficiency",
            description:
              "You can build the map and check complements in a single pass, not two.",
            hints: [
              "Do you need to see all numbers before you start checking?",
              "What if you check for the complement before inserting the current number? When would this fail?",
              "Insert each number after checking its complement. This ensures you never pair a number with itself, and you find the answer in one pass.",
            ],
          },
          {
            order: 4,
            title: "Index tracking",
            description:
              "The map must store indices (not just values) to produce the required output.",
            hints: [
              "What do you need to return besides finding the pair?",
              "If you find a complement in your map, what information do you need stored with it?",
              "Store value→index mappings in the hash map so you can immediately return both indices when a complement match is found.",
            ],
          },
        ],
      },
    ],
  },
  {
    title: "Maximum Subarray Sum",
    difficulty: 1200,
    topic: "dp, greedy",
    description:
      "Given an array of n integers (possibly negative), find the contiguous subarray with the maximum sum.",
    inputSpec: "First line: n. Second line: n space-separated integers.",
    outputSpec: "The maximum subarray sum.",
    examples: [
      { input: "9\n-2 1 -3 4 -1 2 1 -5 4", output: "6" },
      { input: "1\n-1", output: "-1" },
    ],
    routes: [
      {
        name: "Kadane's Algorithm",
        description:
          "Maintain a running sum, resetting when it goes negative. Track the global maximum.",
        order: 1,
        observations: [
          {
            order: 1,
            title: "Subproblem structure",
            description:
              "Define the subproblem as 'maximum subarray ending at position i'.",
            hints: [
              "Can you break this into smaller problems based on where the subarray ends?",
              "What if you knew the best subarray ending at every position? How would that help?",
              "Let dp[i] = maximum sum of a subarray that ends at index i. The answer is max(dp[0..n]).",
            ],
          },
          {
            order: 2,
            title: "Reset condition",
            description:
              "When the running sum goes negative, discard it and start fresh.",
            hints: [
              "What happens when your accumulated sum becomes negative?",
              "If your running sum is negative, can it ever help a future subarray? What should you do?",
              "A negative running sum only hurts future subarrays. Reset to 0 (or to the current element) when the sum becomes negative.",
            ],
          },
          {
            order: 3,
            title: "Single pass O(n)",
            description:
              "The recurrence dp[i] = max(a[i], dp[i-1] + a[i]) gives O(n) time.",
            hints: [
              "How many times do you need to look at each element?",
              "At each position, you have two choices: extend the previous subarray or start new. Which formula captures this?",
              "dp[i] = max(a[i], dp[i-1] + a[i]). Either start a new subarray at i, or extend the previous best. One pass, O(n) time, O(1) space.",
            ],
          },
          {
            order: 4,
            title: "All-negative edge case",
            description:
              "When all elements are negative, the answer is the least negative element.",
            hints: [
              "What if every element is negative? What should the answer be?",
              "An empty subarray isn't valid. If all numbers are negative, what's the best you can do?",
              "Initialize your global max to the first element (not 0 or -infinity depending on variant). This ensures all-negative arrays return the maximum single element.",
            ],
          },
        ],
      },
      {
        name: "Divide and Conquer",
        description:
          "Split at the midpoint. The answer is in the left half, right half, or crosses the midpoint.",
        order: 2,
        observations: [
          {
            order: 1,
            title: "Three-case decomposition",
            description:
              "The maximum subarray either lies entirely in the left half, entirely in the right half, or crosses the midpoint.",
            hints: [
              "If you split the array in half, where could the answer be?",
              "Any subarray must either be fully left, fully right, or span across the middle. Can you solve each case?",
              "Recursively solve left and right halves. The crossing case needs special treatment.",
            ],
          },
          {
            order: 2,
            title: "Crossing subarray computation",
            description:
              "Find max crossing sum by extending from the midpoint in both directions.",
            hints: [
              "How would you find the best subarray that includes the midpoint?",
              "Starting from the midpoint, extend left as far as beneficial, then extend right. How do these combine?",
              "Scan left from mid to find max left sum, scan right from mid+1 to find max right sum. The crossing max is their sum.",
            ],
          },
          {
            order: 3,
            title: "Recurrence and complexity",
            description:
              "T(n) = 2T(n/2) + O(n) gives O(n log n) by the Master Theorem.",
            hints: [
              "What's the time complexity of this approach?",
              "You make 2 recursive calls on halves, plus O(n) work for the crossing case. What does the Master Theorem say?",
              "T(n) = 2T(n/2) + O(n) → O(n log n) by case 2 of the Master Theorem. Slower than Kadane's but demonstrates the technique.",
            ],
          },
          {
            order: 4,
            title: "Base case",
            description:
              "A single-element subarray is its own maximum subarray.",
            hints: [
              "When does the recursion stop?",
              "What's the maximum subarray of a single element?",
              "Base case: when the subarray has one element, return that element. This handles the recursion termination.",
            ],
          },
        ],
      },
    ],
  },
  {
    title: "Binary Search on Answer",
    difficulty: 1300,
    topic: "binary search",
    description:
      "You have n ropes of given lengths. You need to cut k pieces of equal length from them. What is the maximum length of each piece?",
    inputSpec:
      "First line: n and k. Second line: n space-separated rope lengths.",
    outputSpec:
      "Maximum length of each piece (integer, round down).",
    examples: [
      { input: "4 11\n802 743 457 539", output: "200" },
    ],
    routes: [
      {
        name: "Binary Search on the Answer",
        description:
          "Binary search on the piece length. For each candidate length, check if we can cut at least k pieces.",
        order: 1,
        observations: [
          {
            order: 1,
            title: "Monotonic feasibility",
            description:
              "If length L works (can cut ≥ k pieces), then any length < L also works. This monotonicity enables binary search.",
            hints: [
              "If you can cut pieces of length 200, can you cut pieces of length 199?",
              "Is there a relationship between the piece length and whether it's feasible? What kind of relationship?",
              "The feasibility function is monotonically decreasing: shorter pieces → more pieces. This means there's a threshold length where it switches from feasible to infeasible.",
            ],
          },
          {
            order: 2,
            title: "Search space bounds",
            description:
              "Lower bound is 1 (or 0), upper bound is max(rope lengths). Binary search between them.",
            hints: [
              "What's the smallest piece you could cut? The largest?",
              "The answer must be between what two values? How does the maximum rope length factor in?",
              "Search from lo=1 to hi=max(ropes). The answer can't exceed the longest rope. Binary search narrows this range in O(log(max_length)) steps.",
            ],
          },
          {
            order: 3,
            title: "Feasibility check function",
            description:
              "For a given length L, count total pieces by summing floor(rope_i / L) for each rope.",
            hints: [
              "Given a candidate length, how do you check if it works?",
              "How many pieces of length L can you get from a single rope of length R?",
              "count(L) = sum(floor(rope_i / L) for all ropes). If count(L) >= k, then L is feasible. This check is O(n).",
            ],
          },
          {
            order: 4,
            title: "Binary search termination",
            description:
              "Use lo <= hi with mid = (lo + hi + 1) / 2 (ceiling division to avoid infinite loop when searching for max).",
            hints: [
              "How do you update lo and hi after the feasibility check?",
              "When searching for the maximum feasible value, which bound do you update and how?",
              "If feasible(mid): lo = mid. Else: hi = mid - 1. Use mid = (lo + hi + 1) / 2 to avoid infinite loop. When lo == hi, that's the answer.",
            ],
          },
          {
            order: 5,
            title: "Overall complexity",
            description:
              "O(n * log(max_length)): binary search does O(log(max_length)) iterations, each checking O(n) ropes.",
            hints: [
              "What's the total time complexity?",
              "How many binary search iterations? How much work per iteration?",
              "Binary search: O(log(max_rope_length)) iterations. Each iteration: O(n) feasibility check. Total: O(n * log(max_length)).",
            ],
          },
        ],
      },
    ],
  },
  {
    title: "BFS Shortest Path",
    difficulty: 1300,
    topic: "graphs, bfs",
    description:
      "Given an unweighted undirected graph with n nodes and m edges, find the shortest path from node 1 to node n. Output the path length and the path itself.",
    inputSpec:
      "First line: n and m. Next m lines: two integers u, v representing an edge.",
    outputSpec:
      "First line: path length (number of edges), or -1 if no path. Second line: the path as space-separated node numbers.",
    examples: [
      { input: "5 6\n1 2\n1 3\n2 4\n3 4\n4 5\n2 5", output: "2\n1 2 5" },
    ],
    routes: [
      {
        name: "BFS with Parent Tracking",
        description:
          "BFS from node 1, tracking parent pointers to reconstruct the path.",
        order: 1,
        observations: [
          {
            order: 1,
            title: "BFS guarantees shortest path in unweighted graphs",
            description:
              "BFS explores nodes in order of distance. The first time a node is reached, it's via the shortest path.",
            hints: [
              "Why is BFS the right algorithm for unweighted shortest paths?",
              "In what order does BFS discover nodes? What does this imply about distances?",
              "BFS processes nodes level by level (distance 0, then 1, then 2, ...). The first time any node is dequeued, it's at its true shortest distance from the source.",
            ],
          },
          {
            order: 2,
            title: "Parent tracking for path reconstruction",
            description:
              "Store parent[v] = u when discovering v from u. Backtrack from target to source to recover the path.",
            hints: [
              "BFS gives you distances, but how do you find the actual path?",
              "When you discover a new node, what information should you record to later reconstruct the path?",
              "Maintain a parent array. When node v is first reached from node u, set parent[v] = u. After BFS, backtrack from n to 1 using parent pointers.",
            ],
          },
          {
            order: 3,
            title: "Visited array prevents revisiting",
            description:
              "Mark nodes as visited when enqueued (not when dequeued) to avoid duplicate queue entries.",
            hints: [
              "How do you prevent processing the same node multiple times?",
              "When should you mark a node as visited: when you add it to the queue or when you remove it?",
              "Mark visited when enqueuing (not dequeuing). This prevents the same node from being added to the queue multiple times via different edges.",
            ],
          },
          {
            order: 4,
            title: "Adjacency list representation",
            description:
              "Store the graph as an adjacency list for efficient O(n + m) BFS traversal.",
            hints: [
              "How should you represent the graph for efficient traversal?",
              "What data structure lets you quickly find all neighbors of a node?",
              "Use an adjacency list (array of vectors). For each edge (u, v), add v to adj[u] and u to adj[v]. BFS then runs in O(n + m).",
            ],
          },
          {
            order: 5,
            title: "Handle no-path case",
            description:
              "If node n is never reached (parent[n] not set), output -1.",
            hints: [
              "What if the graph is disconnected?",
              "How do you detect that there's no path from 1 to n?",
              "After BFS completes, check if node n was visited. If not, the graph is disconnected between 1 and n — output -1.",
            ],
          },
        ],
      },
    ],
  },
  {
    title: "Coin Change (Minimum Coins)",
    difficulty: 1400,
    topic: "dp",
    description:
      "Given n coin denominations and a target amount, find the minimum number of coins needed to make that amount. Each denomination can be used unlimited times.",
    inputSpec:
      "First line: n and target. Second line: n coin denominations.",
    outputSpec:
      "Minimum number of coins, or -1 if impossible.",
    examples: [
      { input: "3 11\n1 5 6", output: "2" },
      { input: "2 3\n2 4", output: "-1" },
    ],
    routes: [
      {
        name: "Bottom-up DP",
        description:
          "Build dp[amount] = min coins for each amount from 0 to target.",
        order: 1,
        observations: [
          {
            order: 1,
            title: "Optimal substructure",
            description:
              "If we use coin c, the remaining problem is amount - c with the same coins available.",
            hints: [
              "If you use one coin, what problem remains?",
              "Suppose you know the answer for all amounts less than the target. How does one coin choice reduce the problem?",
              "Using coin c transforms amount A into amount A-c. If we know min_coins(A-c), then one option for min_coins(A) is 1 + min_coins(A-c).",
            ],
          },
          {
            order: 2,
            title: "Recurrence relation",
            description:
              "dp[a] = min(dp[a - c] + 1) for all coins c where c ≤ a.",
            hints: [
              "How do you combine the choices of which coin to use?",
              "For a given amount, you can try each coin. What's the formula?",
              "dp[a] = min over all coins c (dp[a - c] + 1) where c ≤ a. Try every coin denomination and take the minimum.",
            ],
          },
          {
            order: 3,
            title: "Base case and initialization",
            description:
              "dp[0] = 0 (zero coins for zero amount). Initialize all others to infinity (unreachable).",
            hints: [
              "What's dp[0]? What should impossible amounts be initialized to?",
              "How do you distinguish 'not yet computed' from 'impossible to make'?",
              "dp[0] = 0. Initialize dp[1..target] = infinity (or target + 1). After filling, if dp[target] is still infinity, return -1.",
            ],
          },
          {
            order: 4,
            title: "Bottom-up iteration order",
            description:
              "Iterate amounts from 1 to target. For each amount, try all coins.",
            hints: [
              "In what order should you fill the DP table?",
              "When computing dp[a], what values do you need to already have computed?",
              "Fill dp[1], dp[2], ..., dp[target] in order. Each dp[a] depends on dp[a-c] for various coins c, all of which are smaller amounts already computed.",
            ],
          },
          {
            order: 5,
            title: "Time and space complexity",
            description:
              "O(target × n) time, O(target) space.",
            hints: [
              "How much work per amount? How many amounts?",
              "For each of the 'target' amounts, how many coins do you try?",
              "For each amount (target values), try each coin (n options). Total: O(target × n) time. Only one 1D array needed: O(target) space.",
            ],
          },
        ],
      },
    ],
  },
  {
    title: "Merge Intervals",
    difficulty: 1200,
    topic: "sorting, greedy",
    description:
      "Given n intervals [start, end], merge all overlapping intervals and return the result.",
    inputSpec:
      "First line: n. Next n lines: two integers start_i and end_i.",
    outputSpec:
      "The merged intervals, one per line.",
    examples: [
      {
        input: "4\n1 3\n2 6\n8 10\n15 18",
        output: "1 6\n8 10\n15 18",
      },
    ],
    routes: [
      {
        name: "Sort and Sweep",
        description:
          "Sort intervals by start time, then sweep left-to-right merging overlapping intervals.",
        order: 1,
        observations: [
          {
            order: 1,
            title: "Sorting enables sequential processing",
            description:
              "Sorting by start time ensures that overlapping intervals are adjacent.",
            hints: [
              "If intervals are in random order, how hard is it to find overlaps?",
              "What ordering of intervals makes it easiest to detect overlaps?",
              "Sort by start time. After sorting, any interval that overlaps with the current merged interval must come immediately after it — no need to look further.",
            ],
          },
          {
            order: 2,
            title: "Overlap detection condition",
            description:
              "Interval B overlaps with current merged interval A if B.start ≤ A.end.",
            hints: [
              "When do two sorted intervals overlap?",
              "If the current merged interval ends at time E, what does the next interval's start need to be for overlap?",
              "After sorting, interval [s2, e2] overlaps with [s1, e1] iff s2 ≤ e1. Since intervals are sorted, s2 ≥ s1 is guaranteed.",
            ],
          },
          {
            order: 3,
            title: "Merge by extending end time",
            description:
              "When overlapping, merge by taking max(current.end, next.end). When not overlapping, start a new interval.",
            hints: [
              "When two intervals overlap, what does the merged interval look like?",
              "How do you combine two overlapping intervals into one?",
              "Merged interval: [min(s1,s2), max(e1,e2)]. Since sorted, min start is s1. So just extend: current.end = max(current.end, next.end).",
            ],
          },
          {
            order: 4,
            title: "O(n log n) from sorting dominance",
            description:
              "Sorting is O(n log n), the sweep is O(n). Total: O(n log n).",
            hints: [
              "What's the bottleneck in the overall time complexity?",
              "The sweep itself is linear. What about the sort?",
              "Sort: O(n log n). Single sweep: O(n). Total: O(n log n), dominated by the sort.",
            ],
          },
        ],
      },
    ],
  },
  {
    title: "Longest Increasing Subsequence",
    difficulty: 1500,
    topic: "dp, binary search",
    description:
      "Given an array of n integers, find the length of the longest strictly increasing subsequence.",
    inputSpec: "First line: n. Second line: n space-separated integers.",
    outputSpec: "Length of the longest increasing subsequence.",
    examples: [
      { input: "8\n10 9 2 5 3 7 101 18", output: "4" },
    ],
    routes: [
      {
        name: "O(n²) DP",
        description:
          "For each position i, compute the LIS ending at i by checking all previous positions.",
        order: 1,
        observations: [
          {
            order: 1,
            title: "Subproblem: LIS ending at position i",
            description:
              "Define dp[i] = length of longest increasing subsequence ending at index i.",
            hints: [
              "What's a natural subproblem to define?",
              "What if you knew the LIS ending at every position before i?",
              "Let dp[i] = length of LIS that ends at index i. The final answer is max(dp[0..n]).",
            ],
          },
          {
            order: 2,
            title: "Transition: check all previous elements",
            description:
              "dp[i] = 1 + max(dp[j]) for all j < i where a[j] < a[i].",
            hints: [
              "To compute dp[i], which earlier positions could the subsequence extend from?",
              "For element a[i], which previous elements could come before it in an increasing subsequence?",
              "dp[i] = 1 + max(dp[j] for j < i if a[j] < a[i]). If no such j exists, dp[i] = 1 (start new subsequence).",
            ],
          },
          {
            order: 3,
            title: "Base case: every element is a subsequence of length 1",
            description: "Initialize dp[i] = 1 for all i.",
            hints: [
              "What's the minimum possible LIS ending at any position?",
              "Even if no previous element is smaller, what's dp[i]?",
              "Every single element is an increasing subsequence of length 1. Initialize all dp[i] = 1.",
            ],
          },
          {
            order: 4,
            title: "O(n²) time from nested loops",
            description:
              "Outer loop over i, inner loop over j < i. Total: O(n²).",
            hints: [
              "What's the time complexity of this approach?",
              "For each i, how many j values do you check?",
              "Outer loop: n iterations. Inner loop: up to i iterations. Total: O(n²). Good enough for n ≤ 5000.",
            ],
          },
          {
            order: 5,
            title: "Answer is the maximum dp value",
            description:
              "The LIS might end at any position, so the answer is max(dp[0..n]).",
            hints: [
              "Where in the dp array is the final answer?",
              "Does the LIS necessarily end at the last position?",
              "The LIS can end anywhere. Scan all dp values and return the maximum.",
            ],
          },
        ],
      },
      {
        name: "O(n log n) with Patience Sorting",
        description:
          "Maintain a tails array where tails[i] is the smallest tail element of all increasing subsequences of length i+1.",
        order: 2,
        observations: [
          {
            order: 1,
            title: "Tails array invariant",
            description:
              "Maintain an array where tails[i] = smallest possible tail element of any increasing subsequence of length i+1.",
            hints: [
              "Can you maintain a structure that tells you the 'best' subsequence of each length?",
              "For subsequences of a given length, which tail element is most useful for extending?",
              "Keep tails[i] = smallest tail of any IS of length i+1. This array is always sorted, which enables binary search.",
            ],
          },
          {
            order: 2,
            title: "Binary search insertion point",
            description:
              "For each element, binary search in tails to find where it fits.",
            hints: [
              "When you process a new element, how do you update the tails array?",
              "The tails array is sorted. What operation on a sorted array is efficient?",
              "Binary search for the first element in tails that is ≥ current element. Replace it (or append if current is larger than all).",
            ],
          },
          {
            order: 3,
            title: "Append extends LIS length",
            description:
              "If the current element is larger than all tails, append it — the LIS just got longer.",
            hints: [
              "When does the LIS length increase?",
              "What happens when the current element is bigger than every tail?",
              "If a[i] > tails.last(), append a[i]. The tails array grows by 1, meaning we found a longer increasing subsequence.",
            ],
          },
          {
            order: 4,
            title: "Length is tails.length, O(n log n) total",
            description:
              "The final LIS length is the length of the tails array. Each element does O(log n) work.",
            hints: [
              "What's the answer when you've processed all elements?",
              "How much work per element? How many elements?",
              "Answer = tails.length. Each of n elements requires O(log n) binary search. Total: O(n log n).",
            ],
          },
        ],
      },
    ],
  },
  {
    title: "Number of Islands",
    difficulty: 1300,
    topic: "graphs, dfs",
    description:
      "Given a 2D grid of '1's (land) and '0's (water), count the number of islands. An island is a group of connected '1's (horizontal/vertical adjacency).",
    inputSpec:
      "First line: rows r and columns c. Next r lines: c characters each ('0' or '1').",
    outputSpec: "Number of islands.",
    examples: [
      {
        input: "4 5\n11110\n11010\n11000\n00000",
        output: "1",
      },
      {
        input: "4 5\n11000\n11000\n00100\n00011",
        output: "3",
      },
    ],
    routes: [
      {
        name: "DFS Flood Fill",
        description:
          "Iterate over the grid. When you find a '1', run DFS to mark the entire island, and increment the count.",
        order: 1,
        observations: [
          {
            order: 1,
            title: "Grid as implicit graph",
            description:
              "Each '1' cell is a node, edges connect adjacent '1' cells. Connected components = islands.",
            hints: [
              "How is this grid problem related to graphs?",
              "What are the nodes and edges in this problem?",
              "Each land cell is a graph node. Adjacent land cells (up/down/left/right) are connected by edges. Each island is a connected component.",
            ],
          },
          {
            order: 2,
            title: "DFS marks entire component",
            description:
              "Starting DFS from any unvisited '1' explores the entire connected island.",
            hints: [
              "If you start exploring from one land cell, what do you find?",
              "How does DFS help you identify all cells belonging to one island?",
              "DFS from a land cell visits every reachable land cell — the entire island. Mark visited cells to avoid revisiting.",
            ],
          },
          {
            order: 3,
            title: "Count = number of DFS launches",
            description:
              "Each time you launch a new DFS from an unvisited '1', that's a new island.",
            hints: [
              "When do you increment your island count?",
              "How many times do you start a fresh DFS?",
              "Scan the grid. Each time you hit an unvisited '1', launch DFS (marking the whole island) and count++. Total count = number of islands.",
            ],
          },
          {
            order: 4,
            title: "In-place marking as visited",
            description:
              "Set visited cells to '0' (or a sentinel) to avoid a separate visited array.",
            hints: [
              "Do you need a separate visited structure?",
              "What if you modify the grid itself to track visited cells?",
              "Change '1' to '0' when visiting. This avoids allocating a visited array and prevents revisiting. The grid is consumed but that's fine if we don't need it later.",
            ],
          },
        ],
      },
    ],
  },
  {
    title: "Sliding Window Maximum",
    difficulty: 1600,
    topic: "deque, sliding window",
    description:
      "Given an array of n integers and a window size k, find the maximum value in each window of size k as it slides from left to right.",
    inputSpec:
      "First line: n and k. Second line: n space-separated integers.",
    outputSpec:
      "n - k + 1 space-separated maximum values.",
    examples: [
      {
        input: "8 3\n1 3 -1 -3 5 3 6 7",
        output: "3 3 5 5 6 7",
      },
    ],
    routes: [
      {
        name: "Monotone Deque",
        description:
          "Maintain a deque of indices where values are monotonically decreasing. The front is always the window maximum.",
        order: 1,
        observations: [
          {
            order: 1,
            title: "Brute force inadequacy",
            description:
              "Naive approach checks all k elements per window: O(nk). We need O(n).",
            hints: [
              "What's wrong with checking all k elements for each window?",
              "For large n and k, O(nk) is too slow. Can we reuse information from the previous window?",
              "Each window shares k-1 elements with the previous one. We're recomputing almost everything. Need a structure that updates in O(1) per slide.",
            ],
          },
          {
            order: 2,
            title: "Deque stores candidate indices",
            description:
              "The deque holds indices of elements that could be the maximum of some future window.",
            hints: [
              "Which elements in the window could potentially be the maximum?",
              "If element a[j] > a[i] and j > i, can a[i] ever be a window maximum?",
              "Store indices in a deque. An element can only be useful if no later, larger element exists in the window. This gives a decreasing sequence of 'candidates'.",
            ],
          },
          {
            order: 3,
            title: "Monotone decreasing invariant",
            description:
              "Before adding a new index, pop all deque elements from the back that are ≤ the new element.",
            hints: [
              "When adding a new element, which existing candidates become useless?",
              "If the new element is larger, can the smaller elements at the back ever win?",
              "Pop from the back while deque.back() value ≤ new value. These elements are now dominated — they'll never be the max because the new element is larger and will be in the window longer.",
            ],
          },
          {
            order: 4,
            title: "Front expiration check",
            description:
              "Remove the front of the deque if its index has left the window (index ≤ i - k).",
            hints: [
              "How do you handle elements that slide out of the window?",
              "The deque front might be an old index. When should you remove it?",
              "Before reading the max, check if deque.front() index is outside the current window (< i - k + 1). If so, pop it. The front is always the current window's max.",
            ],
          },
          {
            order: 5,
            title: "Amortized O(n) complexity",
            description:
              "Each element is pushed and popped from the deque at most once. Total: O(n).",
            hints: [
              "Each pop and push seems expensive, but how many total operations are there?",
              "Can any element be pushed more than once? Popped more than once?",
              "Each of the n elements enters the deque once and leaves at most once. Total operations: O(2n) = O(n), regardless of k.",
            ],
          },
        ],
      },
    ],
  },
  {
    title: "0/1 Knapsack",
    difficulty: 1500,
    topic: "dp",
    description:
      "Given n items with weights and values, and a knapsack with capacity W, find the maximum total value you can carry. Each item can be used at most once.",
    inputSpec:
      "First line: n and W. Next n lines: weight_i and value_i.",
    outputSpec: "Maximum total value.",
    examples: [
      { input: "4 7\n1 1\n3 4\n4 5\n5 7", output: "9" },
    ],
    routes: [
      {
        name: "2D DP (items × capacity)",
        description:
          "Build a table dp[i][w] = max value using items 1..i with capacity w.",
        order: 1,
        observations: [
          {
            order: 1,
            title: "Binary choice per item",
            description:
              "For each item, you either include it or skip it. This gives the recurrence structure.",
            hints: [
              "For any given item, what are your options?",
              "What decision do you make for each item?",
              "Each item is either included (take its value, reduce capacity by its weight) or excluded (capacity unchanged). Two choices per item → binary decision tree → DP.",
            ],
          },
          {
            order: 2,
            title: "State definition: dp[i][w]",
            description:
              "dp[i][w] = maximum value achievable using items 1 through i with knapsack capacity w.",
            hints: [
              "What information do you need to describe a subproblem?",
              "Which items have you considered, and how much capacity remains?",
              "Two dimensions: which items we've decided on (1..i) and remaining capacity (w). dp[i][w] = best value for this subproblem.",
            ],
          },
          {
            order: 3,
            title: "Recurrence: skip or take",
            description:
              "dp[i][w] = max(dp[i-1][w], dp[i-1][w - weight_i] + value_i) if weight_i ≤ w.",
            hints: [
              "How does the choice of including/excluding item i translate to a formula?",
              "If you skip item i, what's the value? If you take it?",
              "Skip: dp[i-1][w]. Take (if weight_i ≤ w): dp[i-1][w - weight_i] + value_i. Choose the max. If weight_i > w, must skip.",
            ],
          },
          {
            order: 4,
            title: "Base case: zero items or zero capacity",
            description:
              "dp[0][w] = 0 for all w. dp[i][0] = 0 for all i.",
            hints: [
              "What happens with no items? With zero capacity?",
              "What are the boundary values of the DP table?",
              "With 0 items, value is 0 regardless of capacity. With 0 capacity, value is 0 regardless of items. Fill these as the base row/column.",
            ],
          },
          {
            order: 5,
            title: "Space optimization to 1D",
            description:
              "Since dp[i] only depends on dp[i-1], use a single 1D array iterated backwards.",
            hints: [
              "Does dp[i][w] depend on anything other than row i-1?",
              "Can you reduce the 2D table to 1D? Be careful about the iteration order.",
              "Use a 1D array dp[0..W]. Iterate items in outer loop, capacity from W down to weight_i in inner loop (backwards to avoid using updated values). O(W) space.",
            ],
          },
          {
            order: 6,
            title: "O(nW) pseudo-polynomial complexity",
            description:
              "Time: O(nW), space: O(W). This is pseudo-polynomial because W is a value, not input size.",
            hints: [
              "What's the time complexity? Is it truly polynomial?",
              "How does the complexity depend on W?",
              "O(nW) time. This is 'pseudo-polynomial' — polynomial in the value of W, not the number of bits needed to represent W. Fine when W ≤ 10^5 or so.",
            ],
          },
        ],
      },
    ],
  },
];
