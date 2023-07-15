export const groupColumn = {
    1: { column: "grid-cols-1", width: "w-[8rem]" },
    2: { column: "grid-cols-2", width: "w-[16rem]" },
    3: { column: "grid-cols-3", width: "w-[24rem]" },
    4: { column: "grid-cols-4", width: "w-[32rem]" },
    5: { column: "grid-cols-5", width: "w-[40rem]" },
    6: { column: "grid-cols-6", width: "w-[48rem]" },
};

export type GroupColumn = keyof typeof groupColumn;
