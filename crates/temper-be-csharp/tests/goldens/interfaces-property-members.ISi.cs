namespace PropertyMembers;

public interface ISi<T>
    where T : II
{
    T Thing { get; }
}
