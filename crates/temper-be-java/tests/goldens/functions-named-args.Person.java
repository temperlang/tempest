package named_args;

import temper.core.Nullable;

final class Person {
    public final String name;
    public final int age;

    public Person(@Nullable String name, @Nullable Integer age) {
        String name2 = name != null ? name : "Alice";
        int age2 = age != null ? age : 40;
        this.name = name2;
        this.age = age2;
    }

    public Person(@Nullable String name) {
        this(name, null);
    }

    public Person() {
        this(null, null);
    }

    public String toString() {
        return "{ class: Person, name: " + this.name + ", age: " + this.age + " }";
    }

    public void greet(@Nullable String greeting) {
        String greeting2 = greeting != null ? greeting : "Hi";
        NamedArgsGlobal.console.log(greeting2 + ", " + this.name);
    }

    public void greet() {
        greet(null);
    }

    public String getName() {
        return this.name;
    }

    public int getAge() {
        return this.age;
    }

    public static final class Builder {
        @Nullable String name;

        public Builder name(@Nullable String name) {
            this.name = name;
            return this;
        }

        @Nullable Integer age;

        public Builder age(@Nullable Integer age) {
            this.age = age;
            return this;
        }

        public Person build() {
            return new Person(name, age);
        }
    }
}
