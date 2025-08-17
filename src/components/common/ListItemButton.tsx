import React from 'react';
import { Button } from "../ui/button";

interface ListItemButtonProps {
    isSelected: boolean;
    onClick: () => void;
    children: React.ReactNode;
    className?: string;
}

const ListItemButton: React.FC<ListItemButtonProps> = ({
    isSelected,
    onClick,
    children,
    className = ""
}) => {
    return (
        <Button
            variant={isSelected ? "default" : "outline"}
            onClick={onClick}
            className={`
                w-full justify-start text-left transition-all duration-200
                ${isSelected
                    ? 'bg-primary hover:bg-primary/90 text-primary-foreground shadow-md'
                    : 'hover:bg-muted hover:border-muted-foreground text-foreground'
                }
                ${className}
            `}
        >
            {children}
        </Button>
    );
};

export default ListItemButton; 