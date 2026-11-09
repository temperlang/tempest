namespace PureVirtual;

internal class IntWrapper : IWrapper<int>
{
    public IntWrapper()
    {
    }

    public int Wrap(int item)
    {
        return item + 1;
    }
}
