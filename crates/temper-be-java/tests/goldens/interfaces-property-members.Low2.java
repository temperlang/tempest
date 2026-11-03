package property_members;

final class Low2 implements Mid {
    public final String s;

    public Low2(String s) {
        this.s = s;
    }

    public String toString() {
        return "Low2(" + this.s + ")";
    }

    public boolean eq(High other) {
        if (other instanceof Low2) {
            return this.s.equals(((Low2) other).s);
        } else {
            return false;
        }
    }

    public String getS() {
        return this.s;
    }
}
