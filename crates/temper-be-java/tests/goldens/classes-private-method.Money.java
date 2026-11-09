package private_method;

final class Money {
    private final int cents;
    private int spent;

    public Money(int cents) {
        this.cents = cents;
        this.spent = 0;
    }

    private int whole() {
        return this.cents / 100;
    }

    private int getRounded() {
        return (this.cents + 50) / 100;
    }

    public Money plus(Money other) {
        return new Money(this.cents + other.cents);
    }

    public boolean sameWhole(Money other) {
        return this.whole() == other.whole();
    }

    public boolean sameRounded(Money other) {
        return this.getRounded() == other.getRounded();
    }

    public void spendFrom(Money other) {
        other.spent = other.spent + 1;
    }

    public int getTimesSpent() {
        return this.spent;
    }

    public String toString() {
        return this.cents + " cents";
    }
}
