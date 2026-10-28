// VietFlow - Vietnamese Input Engine
// Licensed under the GNU General Public License v3.0 (GPLv3)

namespace VietFlow.Windows;

internal sealed class UiOption<T>
{
    public UiOption(T id, string name)
    {
        Id = id;
        Name = name;
    }

    public T Id { get; }
    public string Name { get; }

    public override string ToString() => Name;
}
