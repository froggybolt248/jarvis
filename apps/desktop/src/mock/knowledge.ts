/** Static mock data. Swap for a real notes source once the backend lands. */
export const knowledgeStatus = {
  notesTouchedThisWeek: 3,
};

export interface Note {
  id: string;
  title: string;
  updated: string;
}

export const recentNotes: Note[] = [
  { id: "note-1", title: "Thermo II — entropy notes", updated: "2 hours ago" },
  { id: "note-2", title: "Reading list — systems design", updated: "Yesterday" },
  { id: "note-3", title: "Meeting notes — advisor sync", updated: "3 days ago" },
];
