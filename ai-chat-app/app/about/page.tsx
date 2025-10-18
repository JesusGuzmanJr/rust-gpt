"use client"

import Link from "next/link"
import { ArrowLeft } from "lucide-react"
import { Button } from "@/components/ui/button"

export default function AboutPage() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="border-b border-border">
        <div className="container mx-auto px-6 py-4">
          <Link href="/">
            <Button variant="ghost" size="sm" className="gap-2">
              <ArrowLeft className="h-4 w-4" />
              Back to Chat
            </Button>
          </Link>
        </div>
      </header>

      <main className="container mx-auto px-6 py-12 max-w-4xl">
        <h1 className="text-4xl font-bold mb-6">About AI Chat</h1>

        <div className="prose prose-invert max-w-none space-y-8">
          <section>
            <h2 className="text-2xl font-semibold mb-4">Our Mission</h2>
            <p className="text-muted-foreground leading-relaxed">
              AI Chat is dedicated to providing intelligent, helpful, and accessible conversational AI experiences. We
              believe in the power of AI to enhance human communication and productivity while maintaining transparency
              and user control.
            </p>
          </section>

          <section>
            <h2 className="text-2xl font-semibold mb-4">What We Do</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Our platform offers advanced AI-powered conversations that understand context, provide helpful responses,
              and adapt to your needs. Whether you're looking for information, creative assistance, or problem-solving
              support, AI Chat is here to help.
            </p>
            <ul className="list-disc list-inside space-y-2 text-muted-foreground">
              <li>Natural language understanding and generation</li>
              <li>Context-aware conversations</li>
              <li>Multi-turn dialogue support</li>
              <li>Customizable AI models and parameters</li>
              <li>Secure and private conversations</li>
            </ul>
          </section>

          <section>
            <h2 className="text-2xl font-semibold mb-4">Our Values</h2>
            <div className="grid gap-6 md:grid-cols-2">
              <div className="p-6 rounded-lg bg-accent/50 border border-border">
                <h3 className="text-xl font-semibold mb-2">Transparency</h3>
                <p className="text-muted-foreground">
                  We believe in being open about how our AI works and what it can and cannot do.
                </p>
              </div>
              <div className="p-6 rounded-lg bg-accent/50 border border-border">
                <h3 className="text-xl font-semibold mb-2">Privacy</h3>
                <p className="text-muted-foreground">
                  Your conversations are private and secure. We respect your data and privacy.
                </p>
              </div>
              <div className="p-6 rounded-lg bg-accent/50 border border-border">
                <h3 className="text-xl font-semibold mb-2">Innovation</h3>
                <p className="text-muted-foreground">
                  We continuously improve our AI to provide better, more helpful experiences.
                </p>
              </div>
              <div className="p-6 rounded-lg bg-accent/50 border border-border">
                <h3 className="text-xl font-semibold mb-2">Accessibility</h3>
                <p className="text-muted-foreground">
                  AI should be available to everyone, regardless of technical expertise.
                </p>
              </div>
            </div>
          </section>

          <section>
            <h2 className="text-2xl font-semibold mb-4">Contact Us</h2>
            <p className="text-muted-foreground leading-relaxed">
              Have questions or feedback? We'd love to hear from you. Reach out to us at{" "}
              <a href="mailto:support@aichat.example.com" className="text-primary hover:underline">
                support@aichat.example.com
              </a>
            </p>
          </section>
        </div>
      </main>
    </div>
  )
}
