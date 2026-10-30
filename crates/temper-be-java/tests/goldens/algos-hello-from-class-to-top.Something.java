package hello_from_class_to_top;

final class Something {
    private final String message;

    public Something(String message) {
        this.message = message;
    }

    public void sayMessage() {
        HelloFromClassToTopGlobal.say(this.message);
    }
}
