export function addDays(date: Date, days: number): Date {
    date.setDate(date.getDate() + days);
    return date;
}

export function formatYYYY_MM_DD(date: Date) {
    return date.toLocaleDateString("en-CA");
}

export function formatTime(date: Date | string) {
    if (typeof date === "string") {
        date = new Date(date);
    }

    return date.toLocaleTimeString("en-GB", {
        hour12: false,
        hour: "2-digit",
        minute: "2-digit",
    });
}

export function addMinutes(date: Date, minutes: number) {
    const newDate = new Date(date);
    newDate.setMinutes(date.getMinutes() + minutes);

    return newDate;
}
