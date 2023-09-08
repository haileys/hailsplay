import { PlayerStatus, Queue, TrackInfo } from "./types";

export function getCurrentTrackQueueIndex(player: PlayerStatus, queue: Queue): number | null  {
    let currentTrack = player.track;
    if (currentTrack === null) {
        return null;
    }

    for (let i = 0; i < queue.items.length; i++) {
        if (queue.items[i].id === currentTrack) {
            return i;
        }
    }

    return null;
}

export function getNextTrack(player: PlayerStatus, queue: Queue): TrackInfo | null {
    let currentTrackIndex = getCurrentTrackQueueIndex(player, queue);
    if (currentTrackIndex === null) {
        return null;
    }

    let nextTrackIndex = currentTrackIndex + 1;

    if (nextTrackIndex >= queue.items.length) {
        return null;
    }

    return queue.items[nextTrackIndex].track;
}

export function getPreviousTrack(player: PlayerStatus, queue: Queue): TrackInfo | null {
    let currentTrackIndex = getCurrentTrackQueueIndex(player, queue);
    if (currentTrackIndex === null) {
        return null;
    }

    let previousTrackIndex = currentTrackIndex - 1;

    if (previousTrackIndex < 0) {
        return null;
    }

    return queue.items[previousTrackIndex].track;
}
