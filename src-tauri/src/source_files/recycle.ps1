$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

[ComImport, Guid("947AAB5F-0A5C-4C13-B4D6-4BF7836FC9F8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IRecycleOperation {
    [PreserveSig] int Advise(IntPtr sink, out uint cookie);
    [PreserveSig] int Unadvise(uint cookie);
    [PreserveSig] int SetOperationFlags(uint flags);
    [PreserveSig] int SetProgressMessage([MarshalAs(UnmanagedType.LPWStr)] string message);
    [PreserveSig] int SetProgressDialog(IntPtr dialog);
    [PreserveSig] int SetProperties(IntPtr properties);
    [PreserveSig] int SetOwnerWindow(IntPtr window);
    [PreserveSig] int ApplyPropertiesToItem(IntPtr item);
    [PreserveSig] int ApplyPropertiesToItems(IntPtr items);
    [PreserveSig] int RenameItem(IntPtr item, [MarshalAs(UnmanagedType.LPWStr)] string name, IntPtr sink);
    [PreserveSig] int RenameItems(IntPtr items, [MarshalAs(UnmanagedType.LPWStr)] string name);
    [PreserveSig] int MoveItem(IntPtr item, IntPtr destination, [MarshalAs(UnmanagedType.LPWStr)] string name, IntPtr sink);
    [PreserveSig] int MoveItems(IntPtr items, IntPtr destination);
    [PreserveSig] int CopyItem(IntPtr item, IntPtr destination, [MarshalAs(UnmanagedType.LPWStr)] string name, IntPtr sink);
    [PreserveSig] int CopyItems(IntPtr items, IntPtr destination);
    [PreserveSig] int DeleteItem(IntPtr item, IntPtr sink);
    [PreserveSig] int DeleteItems(IntPtr items);
    [PreserveSig] int NewItem(IntPtr destination, uint attributes, [MarshalAs(UnmanagedType.LPWStr)] string name, [MarshalAs(UnmanagedType.LPWStr)] string template, IntPtr sink);
    [PreserveSig] int PerformOperations();
    [PreserveSig] int GetAnyOperationsAborted([MarshalAs(UnmanagedType.Bool)] out bool aborted);
}

public static class GenModRecycle {
    [DllImport("shell32.dll", CharSet = CharSet.Unicode, PreserveSig = true)]
    static extern int SHCreateItemFromParsingName(string path, IntPtr context, ref Guid iid, out IntPtr item);

    public static void Send(string path) {
        var iid = new Guid("43826D1E-E718-42EE-BC55-A1E261C37BFE");
        IntPtr item = IntPtr.Zero;
        IRecycleOperation operation = null;
        try {
            Marshal.ThrowExceptionForHR(SHCreateItemFromParsingName(path, IntPtr.Zero, ref iid, out item));
            operation = (IRecycleOperation)Activator.CreateInstance(Type.GetTypeFromCLSID(new Guid("3AD05575-8857-4850-9277-11B85BDB8E09")));
            // RECYCLEONDELETE + EARLYFAILURE + NOERRORUI + NOCONFIRMATION + SILENT.
            // Unlike ALLOWUNDO, RECYCLEONDELETE explicitly requests recycling, never a permanent-delete fallback.
            Marshal.ThrowExceptionForHR(operation.SetOperationFlags(0x00180414));
            Marshal.ThrowExceptionForHR(operation.DeleteItem(item, IntPtr.Zero));
            Marshal.ThrowExceptionForHR(operation.PerformOperations());
            bool aborted;
            Marshal.ThrowExceptionForHR(operation.GetAnyOperationsAborted(out aborted));
            if (aborted) { throw new InvalidOperationException("Recycle operation was cancelled."); }
        } finally {
            if (item != IntPtr.Zero) { Marshal.Release(item); }
            if (operation != null) { Marshal.FinalReleaseComObject(operation); }
        }
    }
}
'@
[GenModRecycle]::Send($env:GENMOD_RECYCLE_PATH)
