export default function push_uptime(ctx: ExecutionContext) {
    ctx.waitUntil(
        fetch(
            `https://status.dusterthefirst.com/api/push/0Xy4fZxeFy?status=up&msg=SCHEDULED`
        )
    );
}
