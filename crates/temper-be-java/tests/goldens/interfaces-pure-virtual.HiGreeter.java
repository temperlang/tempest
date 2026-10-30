package pure_virtual;

final class HiGreeter implements Greeter {
    public HiGreeter() {}

    public void greet() {
        PureVirtualGlobal.console.log("Hi, virtually!");
    }
}
