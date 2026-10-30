package inherited_getter;

interface I {
    default String getPlace() {
        return "Byron Bay";
    }
}
