declare module "rehype-sanitize" {
    import { Plugin } from "unified";
    import { Options as SanitizeSchema } from "hast-util-sanitize";

    const rehypeSanitize: Plugin<[SanitizeSchema?]>;

    export default rehypeSanitize;

    export const defaultSchema: SanitizeSchema;
} 