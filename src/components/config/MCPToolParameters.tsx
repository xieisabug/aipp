import React from "react";
import { Collapsible, CollapsibleContent } from "../ui/collapsible";
import MCPToolParameterItem from "./MCPToolParameterItem";

interface MCPToolParametersProps {
    isExpanded: boolean;
    parameters: any;
    truncateText: (text: string, maxLines?: number) => string;
}

const MCPToolParameters: React.FC<MCPToolParametersProps> = ({
    isExpanded,
    parameters,
    truncateText,
}) => {
    if (!parameters || !parameters.properties) {
        return null;
    }

    return (
        <Collapsible open={isExpanded}>
            <CollapsibleContent className="px-4 pb-4">
                <div className="border-t border-border pt-3">
                    <div className="text-sm font-medium text-foreground mb-3">
                        参数：
                    </div>
                    <div className="space-y-3">
                        {Object.entries(parameters.properties).map(
                            ([paramName, paramDef]: [string, any]) => (
                                <MCPToolParameterItem
                                    key={paramName}
                                    paramName={paramName}
                                    paramDef={paramDef}
                                    isRequired={
                                        parameters.required?.includes(
                                            paramName,
                                        ) || false
                                    }
                                    truncateText={truncateText}
                                />
                            ),
                        )}
                    </div>
                </div>
            </CollapsibleContent>
        </Collapsible>
    );
};

export default MCPToolParameters;
