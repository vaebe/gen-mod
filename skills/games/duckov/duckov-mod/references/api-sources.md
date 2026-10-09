# 来源与入口

核对日期：2026-10-09。开发者示例仓库 [xvrsl/duckov_modding](https://github.com/xvrsl/duckov_modding/tree/32721af32edb387757ff536a6e3925f7856c934e)，固定修订 `32721af32edb387757ff536a6e3925f7856c934e`。

- [README](https://github.com/xvrsl/duckov_modding/blob/32721af32edb387757ff536a6e3925f7856c934e/README.md)：加载入口、DLL 和 info.ini 的对应关系。
- [工程参考](https://github.com/xvrsl/duckov_modding/blob/32721af32edb387757ff536a6e3925f7856c934e/DisplayItemValue/DisplayItemValue.csproj)：目标框架与游戏、Unity 程序集引用，引用不复制到产物。
- [入口示例](https://github.com/xvrsl/duckov_modding/blob/32721af32edb387757ff536a6e3925f7856c934e/DisplayItemValue/ModBehaviour.cs)：Duckov.Modding.ModBehaviour 基类和 Unity 生命周期。

本工具采用独立的 ExampleMod 模板，入口为 `ExampleMod.ModBehaviour`；输出 ExampleMod.dll，info.ini name=ExampleMod。仅使用 Unity 的 OnGUI 和 GUI.Label 展示信息，不依赖未经查明的 Duckov 业务成员。参考程序集可能随用户版本变化，编译与游戏运行均尚未验证。以后扩展业务 API 时，以用户实际提供的程序集为准。
