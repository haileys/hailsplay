import { Attributes, Component, ComponentChild, ComponentChildren, Ref, createContext } from "preact";
import { signal } from "@preact/signals";

import ReconnectingWebSocket from "reconnecting-websocket";
import type { Event, ErrorEvent, CloseEvent } from "reconnecting-websocket";
import { PlayerStatus, Queue, TrackInfo, ServerMessage } from "./types";
import { useEffect, useState } from "preact/hooks";

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

    onopen(ev: Event) {
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
                break;

            case "player":
                this.signals.player.value = message.player;
                break;
        }
    }

    onerror(ev: ErrorEvent) {
        console.log("websocket error");
    }
}

export function LiveSession(props: { children: ComponentChildren }) {
    const [socket, setSocket] = useState<SocketClient | null>(null);

    useEffect(() => {
        setSocket(new SocketClient());
        return () => {
            if (socket !== null) {
                socket.close();
            }
        }
    }, []);

    let signals = socket !== null ? socket.signals : new Live();

    return (
        <LiveContext.Provider value={signals}>
            {props.children}
        </LiveContext.Provider>
    );
}

export class Live {
    reconnecting = signal<boolean>(false);
    queue = signal<Queue | null>(null);
    player = signal<PlayerStatus | null>(null);
    currentTrack = signal<TrackInfo | null>(null);
}

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
