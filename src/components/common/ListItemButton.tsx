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
                    ? 'bg-gray-800 hover:bg-gray-900 text-white shadow-md'
                    : 'hover:bg-gray-50 hover:border-gray-300 text-gray-700'
                }
                ${className}
            `}
        >
            {children}
        </Button>
    );
};

export default ListItemButton; 