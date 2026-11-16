import raw from './ask-ai.md?raw';

// The prompt people paste into a chatbot. Placeholders are filled from the configured site and repository.
export function askAiPrompt(site: string, repo: string): string {
  return raw.replaceAll('{{SITE}}', site).replaceAll('{{REPO}}', repo).trim() + '\n';
}
