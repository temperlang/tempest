package private_method;

final class C {
    public C() {}

    private int helper(int x) {
        return x + 1;
    }

    public int method1(int x) {
        return this.helper(x * 2);
    }

    public int method2(int x) {
        return this.helper(x * 3);
    }
}
