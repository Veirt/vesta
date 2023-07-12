type ServiceWidth = 1 | 2 | 3 | 4;
type ServiceHeight = 1 | 2 | 3 | 4;

interface Service {
    title: string;
    href: string;
    imgSrc: string;
    width: ServiceWidth;
    height: ServiceHeight;
}

interface VestaConfig {
    [group: string]: {
        name: string;
        columns: 1 | 2 | 3 | 4;
        services: Service[];
    };
}
