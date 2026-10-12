export type ProviderApi = "openai-completions" | "openai-responses" | "anthropic-messages" | "google-generative-ai";

export const providerApis: { value: ProviderApi; label: string }[] = [
  { value: "openai-completions", label: "OpenAI Chat Completions" },
  { value: "openai-responses", label: "OpenAI Responses" },
  { value: "anthropic-messages", label: "Anthropic Messages" },
  { value: "google-generative-ai", label: "Google Generative AI" },
];

export interface ModelDefinition {
  id: string;
  name: string;
  contextWindow: number;
  maxTokens: number;
  reasoning: boolean;
  input: ("text" | "image")[];
  thinkingLevelMap?: Partial<Record<"off" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max", string | null>>;
}

export interface ModelForm extends Omit<ModelDefinition, "contextWindow" | "maxTokens"> {
  contextWindow: number | "";
  maxTokens: number | "";
}

export interface ProviderConfiguration {
  id: string;
  name: string;
  baseUrl: string;
  api: ProviderApi;
  apiKey?: string; // 仅提交新输入的 Key；读取接口不会返回已保存的 Key。
  keyConfigured: boolean;
  models: ModelDefinition[];
}

export interface SaveConfiguration {
  providers: ProviderConfiguration[];
  defaultProvider: string;
  defaultModel: string;
}

export interface ConfigurationStatus extends SaveConfiguration {
  model: string;
  keyConfigured: boolean;
  configurationDirectory: string;
  runtimeVersion: string;
  nodeVersion: string;
  runtimePrepared: boolean;
}
