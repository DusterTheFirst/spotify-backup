export function push_uptime_alive(ctx: ExecutionContext) {
    ctx.waitUntil(
        fetch(
            `https://status.dusterthefirst.com/api/push/0Xy4fZxeFy?status=up&msg=SCHEDULED`,
        ),
    );
}

export function push_uptime_authorized(ctx: ExecutionContext) {
    ctx.waitUntil(
        fetch(
            `https://status.dusterthefirst.com/api/push/b20P6kNDe2?status=up&msg=AUTHORIZED`,
        ),
    );
}
