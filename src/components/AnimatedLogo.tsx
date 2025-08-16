import React, { useEffect, useState } from "react";

export type LogoState = "normal" | "happy" | "working" | "error" | "thinking";

interface AnimatedLogoProps {
    state?: LogoState;
    size?: number;
    className?: string;
    autoAnimate?: boolean;
    onClick?: () => void;
}

// SVG组件定义
const LogoSVG: React.FC<{ state: LogoState; size: number }> = ({
    state,
    size,
}) => {
    const renderEyes = () => {
        switch (state) {
            case "happy":
                return (
                    <>
                        <path
                            d="M82 199.5C82 199.5 104 167 134 199.5"
                            stroke="#111A2D"
                            strokeWidth="10"
                            strokeLinecap="round"
                        />
                        <path
                            d="M170 199.444C170 199.444 192 166.944 222 199.444"
                            stroke="#111A2D"
                            strokeWidth="10"
                            strokeLinecap="round"
                        />
                    </>
                );
            case "error":
                return (
                    <>
                        <g>
                            <path
                                xmlns="http://www.w3.org/2000/svg"
                                d="M217.588 163.897C216.568 163.066 204.703 162.648 197.812 163.372C195.011 163.667 192.869 165.97 190.29 167.844C187.028 170.215 185.239 172.627 183.075 175.128C180.78 177.781 179.464 181.37 178.126 183.872C176.83 186.292 175.754 188.864 174.929 191.366C172.846 197.677 174.716 208.849 176.156 211.35C177.653 213.948 179.86 216.343 182.336 217.704C184.81 219.064 187.279 219.89 199.095 219.999C204.032 220.045 206.855 218.029 210.054 216.361C213.895 214.36 217.168 209.086 218.819 206.373C220.32 203.904 221.088 201.38 220.992 196.287C220.925 192.717 216.364 191.8 213.888 190.754C211.48 189.736 208.944 189.089 202.363 189.083C198.842 189.08 197.605 193.236 196.569 195.737C196.068 196.949 195.948 199.475 196.254 202.491C196.375 203.687 197.172 204.065 198.202 204.277C199.231 204.49 200.659 204.49 201.701 204.284C202.743 204.077 203.355 203.665 203.986 203.241"
                                stroke="#1F1F1F"
                                stroke-width="8"
                                stroke-linecap="round"
                            />
                            <path
                                xmlns="http://www.w3.org/2000/svg"
                                d="M126.588 163.897C125.568 163.066 113.703 162.648 106.812 163.372C104.011 163.667 101.869 165.97 99.2905 167.844C96.0277 170.215 94.239 172.627 92.075 175.128C89.7802 177.781 88.4641 181.37 87.1255 183.872C85.8302 186.292 84.7543 188.864 83.9289 191.366C81.8461 197.677 83.7156 208.849 85.1562 211.35C86.6528 213.948 88.8598 216.343 91.3361 217.704C93.8098 219.064 96.2794 219.89 108.095 219.999C113.032 220.045 115.855 218.029 119.054 216.361C122.895 214.36 126.168 209.086 127.819 206.373C129.32 203.904 130.088 201.38 129.992 196.287C129.925 192.717 125.364 191.8 122.888 190.754C120.48 189.736 117.944 189.089 111.363 189.083C107.842 189.08 106.605 193.236 105.569 195.737C105.068 196.949 104.948 199.475 105.254 202.491C105.375 203.687 106.172 204.065 107.202 204.277C108.231 204.49 109.659 204.49 110.701 204.284C111.743 204.077 112.355 203.665 112.986 203.241"
                                stroke="#1F1F1F"
                                stroke-width="8"
                                stroke-linecap="round"
                            />
                        </g>
                    </>
                );
            case "working":
                return (
                    <>
                        <g>
                            <path
                                d="M91.4322 164L135 193M135 193H82M135 193L91.4322 220"
                                stroke="#1F1F1F"
                                stroke-width="8"
                            />
                            <path
                                d="M218.568 164L175 193M175 193H228M175 193L218.568 220"
                                stroke="#1F1F1F"
                                stroke-width="8"
                            />
                        </g>
                    </>
                );
            case "thinking":
                return (
                    <>
                        <circle
                            cx="196"
                            cy="199"
                            r="26.5"
                            fill="#111A2D"
                            stroke="white"
                        />
                        <circle
                            cx="108"
                            cy="199"
                            r="26.5"
                            fill="#111A2D"
                            stroke="white"
                        />
                        <circle
                            cx="108"
                            cy="199"
                            r="20.5"
                            fill="white"
                            stroke="white"
                        />
                        <circle
                            cx="196"
                            cy="199"
                            r="20.5"
                            fill="white"
                            stroke="white"
                        />
                    </>
                );
            default: // normal
                return (
                    <>
                        <circle
                            cx="196"
                            cy="199"
                            r="26.5"
                            fill="#111A2D"
                            stroke="white"
                        />
                        <circle
                            cx="108"
                            cy="199"
                            r="26.5"
                            fill="#111A2D"
                            stroke="white"
                        />
                    </>
                );
        }
    };

    return (
        <svg
            width={size}
            height={size}
            viewBox="0 0 510 510"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
            className="transition-all duration-300 ease-in-out"
        >
            {/* 主体部分 */}
            <path
                d="M385 17C452.931 17 508 72.069 508 140V295C508 353.542 460.542 401 402 401H265V251.5C275.627 259.355 288.771 264 303 264C338.346 264 367 235.346 367 200C367 164.654 338.346 136 303 136C288.771 136 275.627 140.645 265 148.499V17H385Z"
                fill="black"
            />
            <path
                d="M385 17C452.931 17 508 72.069 508 140V295C508 353.542 460.542 401 402 401H265V251.5C275.627 259.355 288.771 264 303 264C338.346 264 367 235.346 367 200C367 164.654 338.346 136 303 136C288.771 136 275.627 140.645 265 148.499V17H385Z"
                fill="url(#paint0_linear)"
                fillOpacity="0.2"
            />
            <path
                d="M244 143.332C244.948 160.105 262.969 169.76 271.5 165.5C279.435 161.537 291.823 156 302 156C326.3 156 346 175.7 346 200C346 224.3 326.3 244 302 244C285.413 244 276.5 236.5 273 233.5C263.195 231.539 244.331 239.672 244 259.785V401H174.07L91.5 484C91.4905 484.013 83.3811 494.652 78 492.5C72.5621 490.325 74.4993 473.006 74.5 473V395.923C31.8569 382.201 1 342.205 1 295V140C1 72.069 56.069 17 124 17H244V143.332Z"
                fill="white"
            />
            <path
                d="M244 143.332C244.948 160.105 262.969 169.76 271.5 165.5C279.435 161.537 291.823 156 302 156C326.3 156 346 175.7 346 200C346 224.3 326.3 244 302 244C285.413 244 276.5 236.5 273 233.5C263.195 231.539 244.331 239.672 244 259.785V401H174.07L91.5 484C91.4905 484.013 83.3811 494.652 78 492.5C72.5621 490.325 74.4993 473.006 74.5 473V395.923C31.8569 382.201 1 342.205 1 295V140C1 72.069 56.069 17 124 17H244V143.332Z"
                fillOpacity="0.2"
            />

            {/* 动态眼睛部分 */}
            {renderEyes()}

            <defs>
                <linearGradient
                    id="paint0_linear"
                    x1="265"
                    y1="17"
                    x2="446"
                    y2="386.5"
                    gradientUnits="userSpaceOnUse"
                >
                    <stop stopColor="white" />
                    <stop offset="0.759615" />
                    <stop offset="1" />
                </linearGradient>
                <linearGradient
                    id="paint1_linear"
                    x1="22.5"
                    y1="34.5"
                    x2="112.859"
                    y2="415.751"
                    gradientUnits="userSpaceOnUse"
                >
                    <stop stopColor="white" />
                    <stop offset="0.403846" stopColor="#F5F5F5" />
                    <stop offset="0.759615" stopColor="#1A2437" />
                    <stop offset="1" stopColor="#1A2437" />
                </linearGradient>
            </defs>
        </svg>
    );
};

const AnimatedLogo: React.FC<AnimatedLogoProps> = ({
    state = "normal",
    size = 48,
    className = "",
    autoAnimate = false,
    onClick = () => {},
}) => {
    const [currentState, setCurrentState] = useState<LogoState>(state);

    useEffect(() => {
        setCurrentState(state);
    }, [state]);

    // 自动动画模式（可选）
    useEffect(() => {
        if (!autoAnimate) return;

        const interval = setInterval(() => {
            const states: LogoState[] = ["normal", "happy", "thinking"];
            const randomState =
                states[Math.floor(Math.random() * states.length)];
            setCurrentState(randomState);
        }, 3000);

        return () => clearInterval(interval);
    }, [autoAnimate]);

    return (
        <div className={`inline-block ${className}`} onClick={() => onClick()}>
            <LogoSVG state={currentState} size={size} />
        </div>
    );
};

export default AnimatedLogo;
