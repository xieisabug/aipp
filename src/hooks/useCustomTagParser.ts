import { useCallback } from 'react';

interface CustomTagHandlers {
    [key: string]: (match: RegExpExecArray) => string;
}

const STANDARD_CUSTOM_TAGS: CustomTagHandlers = {
    fileattachment: (match: RegExpExecArray) =>
        `\n<fileattachment ${match[1]}></fileattachment>\n`,
    bangwebtomarkdown: (match: RegExpExecArray) =>
        `\n<bangwebtomarkdown ${match[1]}></bangwebtomarkdown>\n`,
    bangweb: (match: RegExpExecArray) => `\n<bangweb ${match[1]}></bangweb>\n`,
};

const MCP_TOOL_CALL_HANDLER = (match: RegExpExecArray) => {
    const content = match[2] || '';
    const serverMatch = content.match(/<server_name>([^<]*)<\/server_name>/);
    const toolMatch = content.match(/<tool_name>([^<]*)<\/tool_name>/);
    const paramsMatch = content.match(/<parameters>([\s\S]*?)<\/parameters>/);

    const serverName = serverMatch ? serverMatch[1].trim() : '';
    const toolName = toolMatch ? toolMatch[1].trim() : '';
    const parameters = paramsMatch ? paramsMatch[1].trim() : '{}';

    return `\n<!-- MCP_TOOL_CALL:${JSON.stringify({ server_name: serverName, tool_name: toolName, parameters })} -->\n`;
};

const ALL_CUSTOM_TAGS: CustomTagHandlers = {
    ...STANDARD_CUSTOM_TAGS,
    mcp_tool_call: MCP_TOOL_CALL_HANDLER,
};

export const useCustomTagParser = () => {
    const parseCustomTags = useCallback((markdown: string) => {
        let result = markdown;

        Object.keys(ALL_CUSTOM_TAGS).forEach((tag) => {
            const completeRegex = new RegExp(
                `<${tag}([^>]*)>([\\s\\S]*?)<\\/${tag}>`,
                'g',
            );
            let match;
            while ((match = completeRegex.exec(markdown)) !== null) {
                const replacement = ALL_CUSTOM_TAGS[tag](match);
                result = result.replace(match[0], replacement);
            }
        });

        return result;
    }, []);

    return { parseCustomTags };
};