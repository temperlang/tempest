namespace Libraries;

public interface ISink<T>
{
    int Take(T e);
}
