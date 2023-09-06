import css from "./LoadingSpinner.module.css";

export function LoadingSpinnerBlock() {
    return (
        <div class={css.block}>
            <LoadingSpinner />
        </div>
    )
}

export function LoadingSpinner() {
    return (
        <div class={css.loadingSpinner}>
            <div class={css.spinner}><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div><div></div></div>
        </div>
    );
}
