export const cardWidth = {
    1: "md:min-w-[7rem] col-span-1",
    2: "md:min-w-[15rem] col-span-2",
    3: "md:min-w-[23rem] col-span-3",
    4: "md:min-w-[31rem] col-span-4",
    5: "md:min-w-[39rem] col-span-5",
    6: "md:min-w-[47rem] col-span-6",
};

export const cardHeight = {
    1: "min-h-[7rem] max-h-[7rem] row-span-1",
    2: "min-h-[15rem] max-h-[15rem] row-span-2",
    3: "min-h-[23rem] max-h-[23rem] row-span-3",
    4: "min-h-[31rem] max-h-[31rem] row-span-4",
    5: "min-h-[39rem] max-h-[39rem] row-span-5",
    6: "min-h-[47rem] max-h-[47rem] row-span-6",
};

export type CardHeight = keyof typeof cardWidth;
export type CardWidth = keyof typeof cardHeight;
