# Privacy

Jarvis is local-first by design.

- **On-device:** LLM inference (Ollama), speech-to-text (whisper.cpp),
  text-to-speech (Piper), embeddings, and all search indexes.
- **Your data:** lives in a plain-markdown vault folder and a local SQLite
  database. Both stay on your machine. Secrets (OAuth tokens, ntfy topic) are
  stored in the Windows Credential Manager, never in files.
- **Network calls:** exactly two, both optional — Google Calendar (through an
  OAuth client *you* create, so only you can access your data) and ntfy push
  notifications (to a random private topic; you can point Jarvis at a
  self-hosted ntfy server).
- **Telemetry:** none. No analytics, no crash reporting, no phoning home.
