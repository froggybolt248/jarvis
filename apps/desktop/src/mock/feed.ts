/** Static mock data for the Quiet Feed — a log of recent Jarvis actions. */
export interface FeedEntry {
  id: string;
  time: string;
  text: string;
}

export const feedEntries: FeedEntry[] = [
  { id: "feed-1", time: "09:12", text: "Logged breakfast — 420 kcal" },
  { id: "feed-2", time: "08:40", text: "Synced calendar — 2 new events" },
  { id: "feed-3", time: "08:05", text: "Reviewed 8 study cards" },
  { id: "feed-4", time: "07:30", text: "Drafted advisor sync agenda" },
  { id: "feed-5", time: "Yesterday", text: "Logged Pull day — 55 min" },
];
