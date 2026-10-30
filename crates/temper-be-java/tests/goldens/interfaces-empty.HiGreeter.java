package empty;

final class HiGreeter implements Greeter {
    public HiGreeter() {}

    public void greet() {
        EmptyGlobal.console.log("Hi!");
    }
}
