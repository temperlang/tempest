namespace PureVirtual;

internal interface IWrapper<T>
{
    T Wrap(T item);
}
