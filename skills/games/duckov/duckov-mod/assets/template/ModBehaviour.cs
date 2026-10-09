using UnityEngine;

namespace ExampleMod
{
    // 由 Duckov MOD 加载器创建；只显示信息，运行效果由用户验证。
    public sealed class ModBehaviour : Duckov.Modding.ModBehaviour
    {
        private void OnGUI()
        {
            GUI.Label(new Rect(16, 16, 300, 32), "ExampleMod · AI-generated MOD");
        }
    }
}
