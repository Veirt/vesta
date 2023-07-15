type CardWidth = import("$lib/components/Card").CardWidth;
type CardHeight = import("$lib/components/Card").CardHeight;

interface ServiceWidget {
    name: string;
    url?: string;
    key?: string;
}

interface Service {
    title: string;
    href: string;
    imgSrc: string;
    width: CardWidth;
    height: CardHeight;
    widget?: ServiceWidget;
}

interface VestaConfig {
    [group: string]: {
        name: string;
        columns: 1 | 2 | 3 | 4;
        services: Service[];
    };
}
