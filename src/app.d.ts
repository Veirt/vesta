// See https://kit.svelte.dev/docs/types#app
// for information about these interfaces
declare global {
    namespace App {
        // interface Error {}
        // interface Locals {}
        // interface PageData {}
        // interface Platform {}
    }

    interface ObjectConstructor {
        groupBy<Item, Key extends PropertyKey>(
            items: Iterable<Item>,
            keySelector: (item: Item, index: number) => Key,
        ): Record<Key, Item[]>;
    }

    interface MapConstructor {
        groupBy<Item, Key>(
            items: Iterable<Item>,
            keySelector: (item: Item, index: number) => Key,
        ): Map<Key, Item[]>;
    }
}

export { };
