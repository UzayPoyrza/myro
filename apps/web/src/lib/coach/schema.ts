import { z } from "zod";

export const CoachResponseSchema = z.object({
  state: z.enum(["approaching", "found", "moving_away", "uncertain"]),
  confidence: z.number().min(0).max(1),
  matched_observation_id: z.string().nullable(),
  coach_message: z.string().min(1).max(500),
  next_action: z.string().min(1).max(200),
  why_signals: z.array(z.string()).min(1).max(5),
});

export type CoachResponse = z.infer<typeof CoachResponseSchema>;

export const FALLBACK_RESPONSE: CoachResponse = {
  state: "uncertain",
  confidence: 0,
  matched_observation_id: null,
  coach_message:
    "Let me think about that differently. Can you tell me more about your approach?",
  next_action: "Describe your current thinking about the problem",
  why_signals: ["response_parse_error"],
};

/**
 * Attempt to extract and validate a CoachResponse from raw LLM output.
 * Handles cases where the LLM wraps JSON in markdown fences or extra text.
 */
export function parseCoachResponse(raw: string): CoachResponse | null {
  // Try direct parse first
  try {
    const parsed = JSON.parse(raw);
    const result = CoachResponseSchema.safeParse(parsed);
    if (result.success) return result.data;
  } catch {
    // Fall through to extraction
  }

  // Try extracting JSON from markdown fences or surrounding text
  const jsonMatch = raw.match(/\{[\s\S]*\}/);
  if (jsonMatch) {
    try {
      const parsed = JSON.parse(jsonMatch[0]);
      const result = CoachResponseSchema.safeParse(parsed);
      if (result.success) return result.data;
    } catch {
      // Fall through
    }
  }

  return null;
}
