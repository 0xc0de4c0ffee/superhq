// Pull-to-refresh wrapper for a vertically-scrolling list. The whole
// container is scrollable; overscrolling from the top with a touch
// drag reveals a spinner. Releasing past the threshold fires
// `onRefresh`; releasing before snaps back.
//
// Only responds to `pointerType: "touch"` so mouse wheel / trackpad
// flick don't trigger it on desktop — the explicit Refresh button in
// the header is still the desktop affordance.
//
// `overscroll-behavior: contain` disables the browser's own overscroll
// (iOS Safari's rubber-band, Chrome Android's reload) so they don't
// fight our drag.

import { useEffect, useRef, useState } from "react";

interface Props {
    onRefresh: () => Promise<void> | void;
    /// When true the drag starts ignored — useful when a parent already
    /// has a refresh in flight via the header button.
    disabled?: boolean;
    className?: string;
    children: React.ReactNode;
}

/// Maximum dampened pull distance; past this the drag flattens out.
const MAX_PULL = 140;
/// How far the user has to pull before releasing triggers a refresh.
const THRESHOLD = 72;
/// Resistance factor — the drag feels heavy, preventing accidental
/// triggers on short downward gestures.
const RESISTANCE = 0.55;

export default function PullToRefresh({
    onRefresh,
    disabled,
    className = "",
    children,
}: Props) {
    const scrollRef = useRef<HTMLDivElement | null>(null);
    const dragStartY = useRef<number | null>(null);
    const pullRef = useRef(0);
    const refreshingRef = useRef(false);
    const onRefreshRef = useRef(onRefresh);
    const disabledRef = useRef(!!disabled);

    const [pull, setPull] = useState(0);
    const [refreshing, setRefreshing] = useState(false);

    // Mirror the latest callback / flag into refs so the pointer
    // handlers don't need to re-bind on every render.
    useEffect(() => {
        onRefreshRef.current = onRefresh;
    }, [onRefresh]);
    useEffect(() => {
        disabledRef.current = !!disabled;
    }, [disabled]);

    useEffect(() => {
        const el = scrollRef.current;
        if (!el) return;

        const setBoth = (v: number) => {
            pullRef.current = v;
            setPull(v);
        };

        const onPointerDown = (e: PointerEvent) => {
            if (refreshingRef.current || disabledRef.current) return;
            if (e.pointerType !== "touch") return;
            if (el.scrollTop > 0) return;
            dragStartY.current = e.clientY;
        };

        const onPointerMove = (e: PointerEvent) => {
            if (dragStartY.current == null) return;
            if (el.scrollTop > 0) {
                dragStartY.current = null;
                setBoth(0);
                return;
            }
            const dy = e.clientY - dragStartY.current;
            if (dy <= 0) {
                setBoth(0);
                return;
            }
            setBoth(Math.min(MAX_PULL, dy * RESISTANCE));
        };

        const finishDrag = async () => {
            if (dragStartY.current == null) return;
            dragStartY.current = null;
            const landed = pullRef.current;
            if (landed >= THRESHOLD && !refreshingRef.current) {
                refreshingRef.current = true;
                setRefreshing(true);
                try {
                    await onRefreshRef.current();
                } finally {
                    refreshingRef.current = false;
                    setRefreshing(false);
                    setBoth(0);
                }
            } else {
                setBoth(0);
            }
        };

        el.addEventListener("pointerdown", onPointerDown);
        el.addEventListener("pointermove", onPointerMove);
        window.addEventListener("pointerup", finishDrag);
        window.addEventListener("pointercancel", finishDrag);
        return () => {
            el.removeEventListener("pointerdown", onPointerDown);
            el.removeEventListener("pointermove", onPointerMove);
            window.removeEventListener("pointerup", finishDrag);
            window.removeEventListener("pointercancel", finishDrag);
        };
    }, []);

    const indicatorOpacity = Math.min(1, pull / THRESHOLD);
    const indicatorShown = pull > 4 || refreshing;
    const armed = pull >= THRESHOLD || refreshing;

    return (
        <div
            ref={scrollRef}
            className={`relative flex flex-1 flex-col overflow-y-auto ${className}`}
            style={{ overscrollBehavior: "contain" }}
        >
            <div
                className="pointer-events-none absolute top-0 left-1/2 z-10 flex h-10 w-10 -translate-x-1/2 items-center justify-center"
                style={{
                    transform: `translate(-50%, ${pull - 40}px)`,
                    opacity: indicatorShown ? indicatorOpacity : 0,
                    transition: dragStartY.current ? "none" : "all 180ms ease",
                }}
            >
                <svg
                    width="18"
                    height="18"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth={2}
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    className={[
                        "text-app-text-secondary",
                        refreshing ? "animate-spin" : armed ? "rotate-180" : "",
                    ].join(" ")}
                    style={{ transition: "transform 160ms ease" }}
                >
                    <path d="M21 12a9 9 0 1 1-3-6.7" />
                    <polyline points="21 4 21 12 13 12" />
                </svg>
            </div>
            <div
                style={{
                    transform: `translateY(${pull}px)`,
                    transition: dragStartY.current
                        ? "none"
                        : "transform 220ms cubic-bezier(0.32, 0.72, 0, 1)",
                }}
            >
                {children}
            </div>
        </div>
    );
}
