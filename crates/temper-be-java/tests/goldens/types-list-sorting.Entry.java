package sorting;

final class Entry {
    public final int id;
    public final int group;

    public Entry(int id, int group) {
        this.id = id;
        this.group = group;
    }

    public String toString() {
        return "{ id: " + this.id + ", group: " + this.group + " }";
    }

    public int getId() {
        return this.id;
    }

    public int getGroup() {
        return this.group;
    }

    public static final class Builder {
        int id;
        boolean id__set;

        public Builder id(int id) {
            id__set = true;
            this.id = id;
            return this;
        }

        int group;
        boolean group__set;

        public Builder group(int group) {
            group__set = true;
            this.group = group;
            return this;
        }

        public Entry build() {
            if (!id__set || !group__set) {
                StringBuilder _message = new StringBuilder("Missing required fields:");
                if (!id__set) {
                    _message.append(" id");
                }
                if (!group__set) {
                    _message.append(" group");
                }
                throw new IllegalStateException(_message.toString());
            }
            return new Entry(id, group);
        }
    }
}
