# Prompt Templates

LLM prompts used by the Myro coach. Edit these files to tune coaching behavior without recompiling.

## Files

| File | Used by | Description |
|------|---------|-------------|
| `coaching-system.md` | `coaching.rs` | System prompt for real-time coaching interventions |
| `coaching-user.md` | `coaching.rs` | User message template sent with each coaching request |

## Template Variables

Variables use `{{var}}` syntax and are replaced at runtime:

| Variable | Description |
|----------|-------------|
| `{{user_name}}` | User's name from state file |
| `{{problem_title}}` | Current problem title |
| `{{problem_difficulty}}` | Problem difficulty rating |
| `{{problem_description}}` | Full problem statement |
| `{{route_name}}` | Selected solution route name |
| `{{route_description}}` | Route description |
| `{{observations}}` | Formatted observation list with states ([FOUND]/[APPROACHING]/[LOCKED]) |
| `{{code}}` | Current editor content |
| `{{trigger}}` | What triggered this coaching intervention |
| `{{elapsed_secs}}` | Seconds since session started |
| `{{recent_messages}}` | Recent conversation history |
| `{{observations_found}}` | Number of observations unlocked |
| `{{observations_total}}` | Total number of observations |

## Fallback

If template files are missing, compiled-in defaults are used. The defaults match the content of these files at build time.
