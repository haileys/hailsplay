import { Component, ComponentChildren, createContext } from "preact";
import { signal } from "@preact/signals";

import ReconnectingWebSocket from "reconnecting-websocket";
// eslint-disable-next-line no-duplicate-imports
import type { Event, ErrorEvent, CloseEvent } from "reconnecting-websocket";

import { PlayerStatus, Queue, TrackInfo, ServerMessage } from "./types";

export class SocketClient {
    private ws: ReconnectingWebSocket;
    public signals = new Live();

    constructor() {
        this.ws = new ReconnectingWebSocket(websocketUrl().toString());
        this.ws.addEventListener("open", (ev) => this.onopen(ev))
        this.ws.addEventListener("close", (ev) => this.onclose(ev))
        this.ws.addEventListener("message", (ev) => this.onmessage(ev))
        this.ws.addEventListener("error", (ev) => this.onerror(ev))
    }

    close() {
        this.ws.close();
    }

    onopen(_: Event) {
        this.signals.reconnecting.value = false;
        console.log("websocket open");
    }

    onclose(ev: CloseEvent) {
        this.signals.reconnecting.value = true;
        console.log("websocket close: ", ev);
    }

    onmessage(ev: MessageEvent) {
        let message: ServerMessage = JSON.parse(ev.data);

        console.log("websocket message:", message);

        switch (message.t) {
            case "queue":
                this.signals.queue.value = message.queue;
                break;

            case "track-change":
                this.signals.currentTrack.value = message.track;
                this.signals.optimisticTrack.value = null;
                break;

            case "player":
                this.signals.player.value = message.player;
                break;
        }
    }

    onerror(_: ErrorEvent) {
        console.log("websocket error");
    }
}

export type LiveSessionProps = { children: ComponentChildren };

export class LiveSession extends Component<LiveSessionProps> {
    client: SocketClient;
    signals: Live;

    constructor(props: LiveSessionProps) {
        console.log("LiveSession.constructor");
        super(props);
        this.client = new SocketClient();
        this.signals = this.client.signals;
    }

    componentWillUnmount() {
        console.log("LiveSession.componentWillUnmount");
        this.client.close();
    }

    render() {
        return (
            <LiveContext.Provider value={this.signals}>
                {this.props.children}
            </LiveContext.Provider>
        )
    }
}

export class Live {
    reconnecting = signal<boolean>(false);
    queue = signal<Queue | null>(null);
    player = signal<PlayerStatus | null>(null);

    currentTrack = signal<TrackInfo | null>(null);
    optimisticTrack = signal<OptimisticTrack | null>(null);
}

export type OptimisticTrack =
    | { transition: null, track: TrackInfo }
    | { transition: "next", track: TrackInfo }
    | { transition: "previous", track: TrackInfo }
    ;

// just make one global socket client
export const LiveContext = createContext<Live>(new Live());

function websocketUrl(): URL {
    let origin = websocketOrigin();
    origin.pathname = "/ws";
    return origin
}

function websocketOrigin(): URL {
    let url = new URL(location.origin);

    switch (url.protocol) {
        case "http:":
            url.protocol = "ws:";
            return url;

        case "https:":
            url.protocol = "wss:";
            return url;
    }

    throw new Error("unknown scheme in URL: " + url.protocol);
}
