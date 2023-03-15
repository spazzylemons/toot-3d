#include <citro2d.h>

void C2D_SceneBegin_NotInlined(C3D_RenderTarget *target) {
    C2D_SceneBegin(target);
}

bool C3D_TexInit_NotInlined(C3D_Tex *tex, u16 width, u16 height, GPU_TEXCOLOR format) {
    return C3D_TexInit(tex, width, height, format);
}

void C3D_TexSetFilter_NotInlined(C3D_Tex *tex, GPU_TEXTURE_FILTER_PARAM magFilter, GPU_TEXTURE_FILTER_PARAM minFilter) {
    C3D_TexSetFilter(tex, magFilter, minFilter);
}

void C3D_TexSetWrap_NotInlined(C3D_Tex *tex, GPU_TEXTURE_WRAP_PARAM wrapS, GPU_TEXTURE_WRAP_PARAM wrapT) {
    C3D_TexSetWrap(tex, wrapS, wrapT);
}

bool C2D_DrawImageAt_NotInlined(C2D_Image img, float x, float y, float depth, const C2D_ImageTint* tint, float scaleX, float scaleY) {
    return C2D_DrawImageAt(img, x, y, depth, tint, scaleX, scaleY);
}
