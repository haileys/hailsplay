import { Metadata, RadioStation, Id } from "./types";

export type QueueAddResult = {
    mpd_id: Id,
};

export async function queueAdd(url: Url, abortSignal: AbortSignal | null): Promise<QueueAddResult | null> {
    return await post("/api/queue")
        .json({ url })
        .signal(abortSignal)
        .response();
}

export async function tuneRadio(url: Url): Promise<null> {
    return await post("/api/radio/tune")
        .json({ url })
        .response();
}

export async function metadata(url: Url, abortSignal: AbortSignal | null): Promise<Metadata | null> {
    return await get("/api/metadata")
        .param("url", url)
        .signal(abortSignal)
        .response();
}

export async function radioStations(): Promise<RadioStation[]> {
    return await get("/api/radio/stations")
        .response()
}

// helpers from here on:

function get(url: string): RequestBuilder {
    return new RequestBuilder("GET", url);
}

function post(url: string): RequestBuilder {
    return new RequestBuilder("POST", url);
}

export class ApiError extends Error {
    constructor(message: string) {
        super(message);
        this.name = "ApiError";
    }
}

class RequestBuilder {
    _url: string;
    _request: RequestInit & { headers: Headers };
    _queryParams: URLSearchParams;

    constructor(method: string, url: string) {
        this._url = url;
        this._queryParams = new URLSearchParams();
        this._request = { method, headers: new Headers() };
    }

    signal(abortSignal: AbortSignal | null): this {
        this._request.signal = abortSignal;
        return this;
    }

    param(name: string, value: string): this {
        this._queryParams.set(name, value);
        return this;
    }

    json(body: object): this {
        this._request.headers.set("content-type", "application/json");
        this._request.body = JSON.stringify(body);
        return this;
    }

    async response(): Promise<any> {
        let url = this._url;
        if (this._queryParams.size > 0) {
            url += "?" + this._queryParams.toString();
        }

        let response = await catchAbortErrors(fetch(url, this._request));

        if (response === null) {
            return null;
        }

        if (response.status === 500) {
            if (response.headers.get("content-type") === "application/json") {
                let errorInfo: { message: string } = await response.json();
                throw new ApiError(errorInfo.message);
            }
        }

        if (response.status >= 400) {
            throw new Error(`${this._request.method} ${this._url} failed: status ${response.status}`);
        }

        return response.json();
    }
};

function catchAbortErrors<T>(promise: Promise<T>): Promise<T | null> {
    return promise.catch((error) => {
        // silence abort errors
        if (error instanceof DOMException) {
            if (error.name === "AbortError") {
                return null;
            }
        }

        throw error;
    });
}
