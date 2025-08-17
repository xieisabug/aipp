import React from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import { Switch } from "../ui/switch";
import {
    Tooltip,
    TooltipContent,
    TooltipTrigger,
    TooltipProvider,
} from "../ui/tooltip";
import { MCPServerTool } from "../../data/MCP";
import MCPToolParameters from "./MCPToolParameters";

interface MCPToolItemProps {
    tool: MCPServerTool;
    isExpanded: boolean;
    onToggleExpansion: (toolId: number) => void;
    onUpdateTool: (
        toolId: number,
        isEnabled: boolean,
        isAutoRun: boolean,
    ) => void;
    truncateText: (text: string, maxLines?: number) => string;
}

const MCPToolItem: React.FC<MCPToolItemProps> = ({
    tool,
    isExpanded,
    onToggleExpansion,
    onUpdateTool,
    truncateText,
}) => {
    const hasParameters =
        tool.parameters &&
        tool.parameters !== "{}" &&
        tool.parameters !== "null";
    const parameters = hasParameters ? JSON.parse(tool.parameters!) : null;
    const truncatedDescription = truncateText(tool.tool_description || "", 2);
    const isDescriptionTruncated =
        tool.tool_description && truncatedDescription !== tool.tool_description;

    return (
        <div className="bg-muted rounded-lg overflow-hidden">
            <div className="flex items-start justify-between p-4 gap-4">
                <div className="flex-1 min-w-0 pr-4">
                    <div className="flex items-center gap-2 mb-2">
                        <div className="font-medium text-foreground truncate">
                            {tool.tool_name}
                        </div>
                        {hasParameters && (
                            <button
                                onClick={() => onToggleExpansion(tool.id)}
                                className="flex-shrink-0 p-1 hover:bg-muted-foreground/20 rounded transition-colors"
                                title={isExpanded ? "收起参数" : "展开参数"}
                            >
                                {isExpanded ? (
                                    <ChevronDown className="h-4 w-4 text-muted-foreground" />
                                ) : (
                                    <ChevronRight className="h-4 w-4 text-muted-foreground" />
                                )}
                            </button>
                        )}
                    </div>
                    {tool.tool_description && (
                        <TooltipProvider delayDuration={1500}>
                            <Tooltip>
                                <TooltipTrigger asChild>
                                    <div className="text-sm text-muted-foreground leading-relaxed">
                                        {isDescriptionTruncated
                                            ? truncatedDescription
                                            : tool.tool_description}
                                    </div>
                                </TooltipTrigger>
                                {isDescriptionTruncated && (
                                    <TooltipContent
                                        side="bottom"
                                        className="max-w-sm"
                                    >
                                        <p className="text-sm">
                                            {tool.tool_description}
                                        </p>
                                    </TooltipContent>
                                )}
                            </Tooltip>
                        </TooltipProvider>
                    )}
                </div>
                <div className="flex items-center gap-6 flex-shrink-0">
                    <div className="flex items-center gap-2">
                        <span className="text-sm text-foreground whitespace-nowrap">
                            启用
                        </span>
                        <Switch
                            checked={tool.is_enabled}
                            onCheckedChange={(checked) =>
                                onUpdateTool(tool.id, checked, tool.is_auto_run)
                            }
                        />
                    </div>
                    <div className="flex items-center gap-2">
                        <span className="text-sm text-foreground whitespace-nowrap">
                            自动运行
                        </span>
                        <Switch
                            checked={tool.is_auto_run}
                            onCheckedChange={(checked) =>
                                onUpdateTool(tool.id, tool.is_enabled, checked)
                            }
                        />
                    </div>
                </div>
            </div>

            {/* 可展开的参数部分 */}
            {hasParameters && (
                <MCPToolParameters
                    isExpanded={isExpanded}
                    parameters={parameters}
                    truncateText={truncateText}
                />
            )}
        </div>
    );
};

export default MCPToolItem;
