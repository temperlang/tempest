package content_tags;

public final class EventParseEffect implements Effect {
    public final Mark event;

    public EventParseEffect(Mark event) {
        this.event = event;
    }

    public void enactOn(Buffer c) {
        c.mark(this.event);
    }

    public Mark getEvent() {
        return this.event;
    }
}
