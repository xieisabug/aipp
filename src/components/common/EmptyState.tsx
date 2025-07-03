import React from 'react';
import { Card, CardContent } from "../ui/card";

interface EmptyStateProps {
    icon: React.ReactNode;
    title: string;
    description: string;
    action?: React.ReactNode;
}

const EmptyState: React.FC<EmptyStateProps> = ({
    icon,
    title,
    description,
    action
}) => {
    return (
        <Card className="border-dashed border-2 border-gray-300 hover:border-gray-400 transition-colors">
            <CardContent className="flex flex-col items-center justify-center py-16">
                <div className="w-16 h-16 bg-gray-100 rounded-full flex items-center justify-center mb-4">
                    {icon}
                </div>
                <h3 className="text-lg font-semibold text-gray-700 mb-2">
                    {title}
                </h3>
                <p className="text-gray-500 text-center mb-8 max-w-md leading-relaxed">
                    {description}
                </p>
                {action}
            </CardContent>
        </Card>
    );
};

export default EmptyState; 