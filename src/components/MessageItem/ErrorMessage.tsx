import React, { useState } from "react";
import IconButton from "../IconButton";
import Copy from "../../assets/copy.svg?react";
import Ok from "../../assets/ok.svg?react";
import { useCopyHandler } from "@/hooks/useCopyHandler";
import { ChevronDown, ChevronRight } from "lucide-react";

interface ErrorMessageProps {
    content: string;
}

const ErrorMessage: React.FC<ErrorMessageProps> = ({ content }) => {
    const [isExpanded, setIsExpanded] = useState(false);
    const { copyIconState, handleCopy } = useCopyHandler(content);

    // Parse error content to extract main message and details
    const parseErrorContent = (content: string) => {
        // 首先检查是否使用了新的分隔符格式
        const delimiter = "|||ERROR_DETAILS|||";
        if (content.includes(delimiter)) {
            const parts = content.split(delimiter);
            if (parts.length === 2) {
                return {
                    mainMessage: parts[0],
                    details: parts[1],
                    hasDetails: true,
                };
            }
        }

        // 兼容旧的JSON格式
        try {
            const parsed = JSON.parse(content);
            if (parsed.message && parsed.details) {
                return {
                    mainMessage: parsed.message,
                    details: parsed.details,
                    hasDetails: true,
                };
            }
        } catch (e) {
            // Not JSON, try to extract details from text
        }

        // Look for patterns that indicate request body information (向后兼容)
        const detailsPatterns = [
            /\[\[extracted_error_body\]\]: (.+)/,
            /\[\[error_response_body\]\]: (.+)/,
            /\[\[empty_post_error_body\]\]: (.+)/,
            /Request body: (.+)/i,
            /Response: (.+)/i,
        ];

        for (const pattern of detailsPatterns) {
            const match = content.match(pattern);
            if (match) {
                const details = match[1];
                const mainMessage = content.replace(pattern, "").trim();

                // Check if details look like JSON or HTML
                const isStructuredDetails =
                    details.startsWith("{") ||
                    details.startsWith("<") ||
                    details.length > 100;

                return {
                    mainMessage: mainMessage || "请求失败",
                    details: details,
                    hasDetails: isStructuredDetails,
                };
            }
        }

        // If content is very long, consider it might have embedded details
        if (content.length > 200) {
            const lines = content.split("\n");
            if (lines.length > 3) {
                return {
                    mainMessage: lines[0],
                    details: lines.slice(1).join("\n"),
                    hasDetails: true,
                };
            }
        }

        return {
            mainMessage: content,
            details: null,
            hasDetails: false,
        };
    };

    const { mainMessage, details, hasDetails } = parseErrorContent(content);

    const formatDetails = (details: string) => {
        try {
            // Try to format as JSON if it's valid JSON
            const parsed = JSON.parse(details);
            return JSON.stringify(parsed, null, 2);
        } catch (e) {
            // Return as-is if not JSON
            return details;
        }
    };

    return (
        <div className="group relative py-4 px-5 rounded-2xl inline-block max-w-[65%] transition-all duration-200 self-start bg-red-50 text-red-800 border border-red-200">
            <div className="flex items-start space-x-3">
                <div className="flex-shrink-0 w-5 h-5 mt-0.5">
                    <svg
                        className="w-5 h-5 text-red-500"
                        fill="currentColor"
                        viewBox="0 0 20 20"
                    >
                        <path
                            fillRule="evenodd"
                            d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z"
                            clipRule="evenodd"
                        />
                    </svg>
                </div>
                <div className="flex-1">
                    <div className="text-sm font-medium text-red-800 mb-1">
                        AI Request Failed
                    </div>
                    <div className="prose prose-sm max-w-none text-red-700">
                        {mainMessage}
                    </div>
                    {hasDetails && (
                        <div className="mt-3">
                            <button
                                onClick={() => setIsExpanded(!isExpanded)}
                                className="flex items-center space-x-1 text-xs text-red-600 hover:text-red-800 transition-colors"
                            >
                                {isExpanded ? (
                                    <ChevronDown className="w-3 h-3" />
                                ) : (
                                    <ChevronRight className="w-3 h-3" />
                                )}
                                <span>
                                    {isExpanded ? "隐藏详情" : "查看详情"}
                                </span>
                            </button>
                            {isExpanded && details && (
                                <div className="mt-2 p-3 bg-red-100 rounded-lg border border-red-200">
                                    <pre className="text-xs text-red-700 whitespace-pre-wrap overflow-x-auto max-h-40 overflow-y-auto">
                                        {formatDetails(details)}
                                    </pre>
                                </div>
                            )}
                        </div>
                    )}
                </div>
            </div>
            <div className="hidden group-hover:flex items-center absolute -bottom-9 py-3 px-4 box-border h-10 rounded-[21px] border border-red-200 bg-red-50 left-0">
                <IconButton
                    icon={
                        copyIconState === "copy" ? (
                            <Copy fill="#dc2626" />
                        ) : (
                            <Ok fill="#dc2626" />
                        )
                    }
                    onClick={handleCopy}
                />
            </div>
        </div>
    );
};

export default ErrorMessage;
