// SPDX-FileCopyrightText: 2020 Sveriges Television AB
//
// SPDX-License-Identifier: Apache-2.0

import static java.awt.RenderingHints.*;

import java.awt.*;
import java.awt.color.ColorSpace;
import java.awt.geom.Rectangle2D;
import java.awt.image.*;
import java.io.File;

import javax.imageio.ImageIO;

class JavaFilterSample {
    private static int onFrameCounter;

    public static void init() {
        System.out.println("init");
    }

    public static byte[] onFrame(byte[] pixels, int width, int height, double ts) {
        System.out.println("onFrame");
        if (onFrameCounter != 0) {
            // We only care about the first frame in this sample ;)
            return null;
        }

        ColorSpace colorSpace = ColorSpace.getInstance(ColorSpace.CS_sRGB);
        ColorModel colorModel = new ComponentColorModel(colorSpace, new int[] {8, 8, 8, 8}, true,
                false, Transparency.TRANSLUCENT, DataBuffer.TYPE_BYTE);

        DataBuffer dataBuffer = new DataBufferByte(pixels, pixels.length);
        WritableRaster raster = Raster.createInterleavedRaster(
                dataBuffer, width, height, width * 4, 4, new int[] {3, 2, 1, 0}, null);

        BufferedImage image = new BufferedImage(colorModel, raster, false, null);
        Graphics2D g2d = image.createGraphics();

        setRenderingHints(g2d);

        Rectangle2D.Double rect = new Rectangle2D.Double(100, 100, 100, 100);
        g2d.setColor(Color.YELLOW);
        g2d.fill(rect);

        BasicStroke stroke =
                new BasicStroke(3.0f, BasicStroke.CAP_SQUARE, BasicStroke.JOIN_ROUND, 10.0f);
        g2d.setStroke(stroke);
        g2d.setColor(Color.MAGENTA);
        g2d.draw(rect);

        g2d.dispose();

        File pngFile = new File("java-sample.png");
        System.out.println("Saving PNG of frame to: " + pngFile.getAbsolutePath());
        try {
            ImageIO.write(image, "PNG", pngFile);
        } catch (Exception e) {
            throw new RuntimeException(e);
        }

        return pixels;
    }

    public static void destroy() {
        System.out.println("destroy");
    }

    private static void setRenderingHints(Graphics2D g2d) {
        g2d.setRenderingHint(KEY_ANTIALIASING, VALUE_ANTIALIAS_ON);
        g2d.setRenderingHint(KEY_COLOR_RENDERING, VALUE_COLOR_RENDER_QUALITY);
        g2d.setRenderingHint(KEY_RENDERING, VALUE_RENDER_QUALITY);
        g2d.setRenderingHint(KEY_TEXT_ANTIALIASING, VALUE_TEXT_ANTIALIAS_ON);
        g2d.setRenderingHint(KEY_STROKE_CONTROL, VALUE_STROKE_PURE);
        g2d.setRenderingHint(KEY_FRACTIONALMETRICS, VALUE_FRACTIONALMETRICS_ON);
    }
}
