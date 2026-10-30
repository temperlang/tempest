package classes;

final class Version {
    public final int major;
    public final int minor;

    public Version(int major, int minor) {
        this.major = major;
        this.minor = minor;
    }

    public int compareTo(Version other) {
        if (this.major != other.major) {
            return Integer.compare(this.major, other.major);
        } else {
            return Integer.compare(this.minor, other.minor);
        }
    }

    public int getMajor() {
        return this.major;
    }

    public int getMinor() {
        return this.minor;
    }

    public static final class Builder {
        int major;
        boolean major__set;

        public Builder major(int major) {
            major__set = true;
            this.major = major;
            return this;
        }

        int minor;
        boolean minor__set;

        public Builder minor(int minor) {
            minor__set = true;
            this.minor = minor;
            return this;
        }

        public Version build() {
            if (!major__set || !minor__set) {
                StringBuilder _message = new StringBuilder("Missing required fields:");
                if (!major__set) {
                    _message.append(" major");
                }
                if (!minor__set) {
                    _message.append(" minor");
                }
                throw new IllegalStateException(_message.toString());
            }
            return new Version(major, minor);
        }
    }
}
