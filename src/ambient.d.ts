type ServiceWidth = 1 | 2 | 3 | 4;
type ServiceHeight = 1 | 2 | 3 | 4;

interface ServiceWidget {
    name: string;
    url?: string;
    key?: string;
}

interface Service {
    title: string;
    href: string;
    imgSrc: string;
    width: ServiceWidth;
    height: ServiceHeight;
    widget?: ServiceWidget;
}

interface VestaConfig {
    [group: string]: {
        name: string;
        columns: 1 | 2 | 3 | 4;
        services: Service[];
    };
}
