import css from "./Player.module.css";

import { LiveContext, OptimisticTrack } from "../socket";
import { useContext } from "preact/hooks";
import PlayerControls from "./PlayerControls";
import { PlayerStatus, Queue, QueueItem, TrackInfo } from "../types";
import { Component, RefObject, createRef } from "preact";

export default function Player() {
    const live = useContext(LiveContext);

    if (live.queue.value === null) {
        return null;
    }

    if (live.player.value == null) {
        return null;
    }

    return (
        <LoadedPlayer
            queue={live.queue.value}
            player={live.player.value}
            optimistic={live.optimisticTrack.value}
        />
    );
}

type LoadedPlayerProps = { queue: Queue, player: PlayerStatus, optimistic: OptimisticTrack | null };

class LoadedPlayer extends Component<LoadedPlayerProps> {
    playerRef: RefObject<HTMLDivElement>;

    constructor(props: LoadedPlayerProps) {
        super(props);
        this.playerRef = createRef();
    }

    render() {
        let { history, queue } = this.partitionedQueue()

        return (
            <>
                <QueueList items={history} scrollSnapStop={true} />
                <div class={css.player} ref={this.playerRef}>
                    <TrackTransitionContainer />
                    <PlayerControls onChangeTrack={() => this.scrollToPlayer()} />
                </div>
                <QueueList items={queue} scrollSnapStop={true} />
            </>
        );
    }

    partitionedQueue(): { history: QueueItem[], queue: QueueItem[] } {
        let i = 0;

        let currentTrackId = this.props.player.track;
        let items = this.props.queue.items;

        // collect all queue items before current into history
        let history = [];
        if (currentTrackId !== null) {
            for (; i < items.length; i++) {
                if (items[i].id === currentTrackId) {
                    break;
                }

                history.push(items[i]);
            }
        }

        // collect and skip past current track
        let currentTrack = i < items.length ? items[i] : null;
        i++;

        // collect all remaining queue items into queue
        let queue = [];
        for (; i < items.length; i++) {
            queue.push(items[i]);
        }

        // if we have a skip forward or skip backward, adjust partitions accordingly
        switch (this.props.optimistic?.transition) {
            case "previous":
                // we skipped backward, remove last item from history
                // and insert current track into the front of the queue
                history.pop();
                if (currentTrack) {
                    queue.unshift(currentTrack)
                }
                break;
            case "next":
                // we skipped forward, remove first item from queue and push
                // current track to the end of history (the Fukuyama Zone)
                queue.shift();
                if (currentTrack) {
                    history.push(currentTrack);
                }
                break;
        }

        return { history, queue };
    }

    componentDidMount(): void {
        this.scrollToPlayer();
    }

    componentDidUpdate(previousProps: Readonly<LoadedPlayerProps>): void {
        let trackChanged = previousProps.player.track !== this.props.player.track;
        let optimisticChanged = previousProps.optimistic !== this.props.optimistic;

        // both of these events can affect scroll position
        if (trackChanged || optimisticChanged) {
            this.scrollToPlayer();
        }
    }

    scrollToPlayer(): void {
        if (this.playerRef.current) {
            this.playerRef.current.scrollIntoView({ behavior: "instant" });
        }
    }
}

function TrackTransitionContainer() {
    const live = useContext(LiveContext);

    let currentTrack = live.currentTrack.value;
    if (!currentTrack) {
        return null;
    }

    let optimistic = live.optimisticTrack.value;
    if (!optimistic?.transition) {
        return (
            <div class={css.trackTransitionContainer}>
                <div key="current-track">
                    <Track track={currentTrack} />
                </div>
            </div>
        );
    }

    switch (optimistic.transition) {
        case "next":
            return (
                <div class={css.trackTransitionContainer}>
                    <div key="current-track" class={css.transitionOutToLeft}>
                        <Track track={currentTrack} />
                    </div>
                    <div key="next-track" class={css.transitionInFromRight}>
                        <Track track={optimistic.track} />
                    </div>
                </div>
            );

        case "previous":
            return (
                <div class={css.trackTransitionContainer}>
                    <div key="previous-track" class={css.transitionInFromLeft}>
                        <Track track={optimistic.track} />
                    </div>
                    <div key="current-track" class={css.transitionOutToRight}>
                        <Track track={currentTrack} />
                    </div>
                </div>
            );
    }
}

function Track(props: { track: TrackInfo }) {
    return (
        <div class={css.trackInfo}>
            <div class={css.coverArtContainer}>
                <div class={css.coverArtInset}>
                    { props.track.imageUrl ? (
                        <img src={props.track.imageUrl} class={css.image} />
                    ) : (
                        // TODO - handle no cover art case
                        null
                    ) }
                </div>
            </div>
            <div>
                <div class={css.trackPrimaryLabel}>
                    {props.track.primaryLabel}
                </div>
                <div class={css.trackSecondaryLabel}>
                    {props.track.secondaryLabel}
                </div>
            </div>
        </div>
    );
}

function QueueList(props: { items: QueueItem[], scrollSnapStop: boolean }) {
    return (
        <>
            {props.items.map(item => (
                <QueueListItem track={item.track} scrollSnapStop={props.scrollSnapStop} key={item.id} />
            ))}
        </>
    )
}

function QueueListItem(props: { track: TrackInfo, scrollSnapStop: boolean }) {
    let itemClassName = props.scrollSnapStop
        ? `${css.queueItem} ${css.scrollSnapStop}`
        : css.queueItem;

    return (
        <div class={itemClassName}>
            <div class={css.queueItemArt}>
                { props.track.imageUrl ? (
                    <img src={props.track.imageUrl} class={css.image} />
                ) : (
                    // TODO - handle no cover art case
                    null
                ) }
            </div>
            <div class={css.queueItemDetails}>
                <div class={css.queueItemPrimaryLabel}>
                    {props.track.primaryLabel}
                </div>
                <div class={css.queueItemSecondaryLabel}>
                    {props.track.secondaryLabel}
                </div>
            </div>
        </div>
    );
}
