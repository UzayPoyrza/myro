export interface LLMMessage {
  role: "user" | "assistant";
  content: string;
}

export interface LLMOptions {
  temperature?: number;
  maxTokens?: number;
}

export interface LLMProvider {
  name: string;
  chat(
    systemPrompt: string,
    messages: LLMMessage[],
    options?: LLMOptions
  ): Promise<string>;
}

export function getProvider(): LLMProvider {
  const providerName = process.env.LLM_PROVIDER || "anthropic";
  const apiKey = process.env.LLM_API_KEY;

  if (!apiKey) {
    throw new Error(
      "LLM_API_KEY environment variable is required. Set it in apps/web/.env"
    );
  }

  switch (providerName) {
    case "anthropic":
      return createAnthropicProvider(apiKey);
    case "openai":
      return createOpenAIProvider(apiKey);
    default:
      throw new Error(
        `Unknown LLM_PROVIDER: ${providerName}. Use "anthropic" or "openai".`
      );
  }
}

function createAnthropicProvider(apiKey: string): LLMProvider {
  return {
    name: "anthropic",
    async chat(systemPrompt, messages, options = {}) {
      const Anthropic = (await import("@anthropic-ai/sdk")).default;
      const client = new Anthropic({ apiKey });

      const response = await client.messages.create({
        model: "claude-sonnet-4-20250514",
        max_tokens: options.maxTokens ?? 1024,
        temperature: options.temperature ?? 0.7,
        system: systemPrompt,
        messages: messages.map((m) => ({
          role: m.role,
          content: m.content,
        })),
      });

      const block = response.content[0];
      if (block.type === "text") return block.text;
      throw new Error("Unexpected response type from Anthropic");
    },
  };
}

function createOpenAIProvider(apiKey: string): LLMProvider {
  return {
    name: "openai",
    async chat(systemPrompt, messages, options = {}) {
      const OpenAI = (await import("openai")).default;
      const client = new OpenAI({ apiKey });

      const response = await client.chat.completions.create({
        model: "gpt-4o",
        max_tokens: options.maxTokens ?? 1024,
        temperature: options.temperature ?? 0.7,
        messages: [
          { role: "system", content: systemPrompt },
          ...messages.map((m) => ({
            role: m.role as "user" | "assistant",
            content: m.content,
          })),
        ],
      });

      return response.choices[0]?.message?.content ?? "";
    },
  };
}
