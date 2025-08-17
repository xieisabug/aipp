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
        <Card className="border-dashed border-2 border-border hover:border-muted-foreground transition-colors">
            <CardContent className="flex flex-col items-center justify-center py-16">
                <div className="w-16 h-16 bg-muted rounded-full flex items-center justify-center mb-4">
                    {icon}
                </div>
                <h3 className="text-lg font-semibold text-foreground mb-2">
                    {title}
                </h3>
                <p className="text-muted-foreground text-center mb-8 max-w-md leading-relaxed">
                    {description}
                </p>
                {action}
            </CardContent>
        </Card>
    );
};

export default EmptyState; 