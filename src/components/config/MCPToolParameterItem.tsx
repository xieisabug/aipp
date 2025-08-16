import React from "react";
import { Badge } from "../ui/badge";
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/tooltip";

interface MCPToolParameterItemProps {
    paramName: string;
    paramDef: any;
    isRequired: boolean;
    truncateText: (text: string, maxLines?: number) => string;
}

const MCPToolParameterItem: React.FC<MCPToolParameterItemProps> = ({
    paramName,
    paramDef,
    isRequired,
    truncateText,
}) => {
    const truncatedParamDesc = truncateText(paramDef.description || "", 2);
    const isParamDescTruncated =
        paramDef.description && truncatedParamDesc !== paramDef.description;

    return (
        <div className="bg-background rounded p-3 border border-border">
            <div className="flex items-start justify-between gap-3">
                <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                        <span className="font-medium text-foreground text-sm">
                            {paramName}
                        </span>
                        {paramDef.type && (
                            <Badge
                                variant="outline"
                                className="text-xs px-2 py-0.5"
                            >
                                {paramDef.type}
                            </Badge>
                        )}
                        {isRequired && (
                            <Badge
                                variant="destructive"
                                className="text-xs px-2 py-0.5"
                            >
                                必需
                            </Badge>
                        )}
                    </div>
                    {paramDef.description && (
                        <Tooltip>
                            <TooltipTrigger asChild>
                                <div className="text-xs text-muted-foreground leading-relaxed">
                                    {isParamDescTruncated
                                        ? truncatedParamDesc
                                        : paramDef.description}
                                </div>
                            </TooltipTrigger>
                            {isParamDescTruncated && (
                                <TooltipContent
                                    side="bottom"
                                    className="max-w-sm"
                                >
                                    <p className="text-xs">
                                        {paramDef.description}
                                    </p>
                                </TooltipContent>
                            )}
                        </Tooltip>
                    )}
                </div>
            </div>
        </div>
    );
};

export default MCPToolParameterItem;
