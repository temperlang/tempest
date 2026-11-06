package java_classes;

public final class Person implements Polite, Named {
    public final String name;
    public final boolean loud;
    public int level;
    public static String banner;
    public static final int count = 3;
    public static final int twice = Person.count * 2;

    public Person(String name, boolean loud) {
        this.name = name;
        this.loud = loud;
        this.level = 0;
    }

    public int getVolume() {
        return this.level;
    }

    public void setVolume(int v) {
        this.level = v;
    }

    public String getName() {
        return this.name;
    }

    public boolean isLoud() {
        return this.loud;
    }

    public int getLevel() {
        return this.level;
    }

    public void setLevel(int newLevel) {
        this.level = newLevel;
    }

    public String greet(int times) {
        return Polite.super.greet(times);
    }

    static {
        JavaClassesGlobal.initStatics();
    }

    public static final class Builder {
        String name;

        public Builder name(String name) {
            this.name = name;
            return this;
        }

        boolean loud;
        boolean loud__set;

        public Builder loud(boolean loud) {
            loud__set = true;
            this.loud = loud;
            return this;
        }

        public Person build() {
            if (name == null || !loud__set) {
                StringBuilder _message = new StringBuilder("Missing required fields:");
                if (name == null) {
                    _message.append(" name");
                }
                if (!loud__set) {
                    _message.append(" loud");
                }
                throw new IllegalStateException(_message.toString());
            }
            return new Person(name, loud);
        }
    }
}
