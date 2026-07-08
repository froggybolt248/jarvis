/** Static mock data. Swap for a real calendar source once the backend lands. */
export interface CalendarEvent {
  id: string;
  time: string;
  title: string;
  location?: string;
}

export const upcomingEvents: CalendarEvent[] = [
  { id: "evt-1", time: "10:00", title: "Thermo II study group", location: "Library, Rm 4" },
  { id: "evt-2", time: "13:30", title: "1:1 with advisor" },
  { id: "evt-3", time: "18:00", title: "Push day — gym" },
];

export const weekEvents: CalendarEvent[] = [
  ...upcomingEvents,
  { id: "evt-4", time: "09:00", title: "Lab report due", location: "Canvas" },
  { id: "evt-5", time: "15:00", title: "Grocery run" },
];
