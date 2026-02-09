import { useState, useEffect } from 'react'
import { ingestionClient } from '../../api/clients'

function IngestionTab({ onResult }) {
  const [jsonData, setJsonData] = useState('')
  const [autoExecute, setAutoExecute] = useState(true)
  const [trustDistance, setTrustDistance] = useState(0)
  const [pubKey, setPubKey] = useState('default')
  const [isLoading, setIsLoading] = useState(false)
  const [ingestionStatus, setIngestionStatus] = useState(null)

  useEffect(() => {
    fetchIngestionStatus()
  }, [])

  const fetchIngestionStatus = async () => {
    try {
      const response = await ingestionClient.getStatus()
      if (response.success) {
        setIngestionStatus(response.data)
      }
    } catch (error) {
      console.error('Failed to fetch ingestion status:', error)
    }
  }

  const processIngestion = async () => {
    setIsLoading(true)
    
    // Clear any previous results
    onResult(null)
    
    try {
      const parsedData = JSON.parse(jsonData)

      const options = {
        autoExecute,
        trustDistance,
        pubKey
      }

      const response = await ingestionClient.processIngestion(parsedData, options)
      
      if (response.success) {
        onResult({
          success: true,
          data: response.data
        })
        setJsonData('') // Clear the form on success
      } else {
        onResult({
          success: false,
          error: 'Failed to process ingestion'
        })
      }
    } catch (error) {
      onResult({
        success: false,
        error: error.message || 'Failed to process ingestion'
      })
    } finally {
      setIsLoading(false)
    }
  }

  const generateBlogPosts = () => {
    const authors = [
      "Sarah Chen", "Michael Rodriguez", "Emily Johnson", "David Kim", "Lisa Wang",
      "James Thompson", "Maria Garcia", "Alex Chen", "Rachel Green", "Tom Wilson",
      "Jennifer Lee", "Chris Anderson", "Amanda Taylor", "Ryan Murphy", "Jessica Brown",
      "Kevin Park", "Nicole Davis", "Brandon White", "Stephanie Martinez", "Daniel Liu"
    ]

    const topics = [
      "Technology", "Programming", "Web Development", "Data Science", "Machine Learning",
      "Artificial Intelligence", "Cloud Computing", "DevOps", "Cybersecurity", "Mobile Development",
      "UI/UX Design", "Product Management", "Startup Life", "Career Advice", "Industry Trends",
      "Open Source", "Software Architecture", "Database Design", "API Development", "Testing"
    ]

    const tags = [
      ["javascript", "webdev", "tutorial"], ["python", "datascience", "ai"],
      ["react", "frontend", "javascript"], ["nodejs", "backend", "api"],
      ["docker", "devops", "deployment"], ["aws", "cloud", "infrastructure"],
      ["machine-learning", "python", "data"], ["typescript", "webdev", "frontend"],
      ["kubernetes", "devops", "containers"], ["sql", "database", "backend"],
      ["git", "version-control", "workflow"], ["testing", "quality", "tdd"],
      ["security", "cybersecurity", "best-practices"], ["performance", "optimization", "web"],
      ["mobile", "ios", "android"], ["design", "ux", "ui"], ["agile", "management", "process"],
      ["career", "advice", "development"], ["startup", "entrepreneurship", "business"],
      ["opensource", "community", "contribution"], ["architecture", "scalability", "design"]
    ]

    const blogPosts = []
    
    for (let i = 1; i <= 100; i++) {
      const author = authors[Math.floor(Math.random() * authors.length)]
      const topic = topics[Math.floor(Math.random() * topics.length)]
      const postTags = tags[Math.floor(Math.random() * tags.length)]
      
      // Generate realistic publish dates over the last 6 months
      const now = new Date()
      const sixMonthsAgo = new Date(now.getTime() - (6 * 30 * 24 * 60 * 60 * 1000))
      const randomTime = sixMonthsAgo.getTime() + Math.random() * (now.getTime() - sixMonthsAgo.getTime())
      const publishDate = new Date(randomTime).toISOString().split('T')[0]
      
      const titles = [
        `Getting Started with ${topic}: A Complete Guide`,
        `Advanced ${topic} Techniques You Need to Know`,
        `Why ${topic} is Changing the Industry`,
        `Building Scalable Applications with ${topic}`,
        `The Future of ${topic}: Trends and Predictions`,
        `Common ${topic} Mistakes and How to Avoid Them`,
        `Best Practices for ${topic} Development`,
        `From Beginner to Expert in ${topic}`,
        `Case Study: Implementing ${topic} in Production`,
        `${topic} Tools and Frameworks Comparison`
      ]
      
      const title = titles[Math.floor(Math.random() * titles.length)]
      
      const contentTemplates = [
        `In this comprehensive guide, we'll explore the fundamentals of ${topic} and how it's revolutionizing the way we approach modern development. Whether you're a seasoned developer or just starting out, this article will provide valuable insights into best practices and real-world applications.

## Introduction to ${topic}

${topic} has become an essential part of today's technology landscape. With its powerful capabilities and growing ecosystem, it offers developers unprecedented opportunities to build robust and scalable solutions.

## Key Concepts

Understanding the core concepts of ${topic} is crucial for success. Let's dive into the fundamental principles that make this technology so powerful:

1. **Core Architecture**: The foundation of ${topic} lies in its well-designed architecture
2. **Performance Optimization**: Learn how to maximize efficiency and minimize resource usage
3. **Integration Patterns**: Discover best practices for connecting with other systems
4. **Security Considerations**: Implement robust security measures from the ground up

## Real-World Applications

Many companies have successfully implemented ${topic} in their production environments. Here are some notable examples:

- **Case Study 1**: A major e-commerce platform reduced their response time by 60%
- **Case Study 2**: A fintech startup improved their scalability by 300%
- **Case Study 3**: A healthcare company enhanced their data processing capabilities

## Getting Started

Ready to dive in? Here's a step-by-step guide to get you started with ${topic}:

\`\`\`javascript
// Example implementation
const example = new ${topic}();
example.initialize();
example.process();
\`\`\`

## Conclusion

${topic} represents a significant advancement in technology, offering developers powerful tools to build the next generation of applications. By following the principles and practices outlined in this guide, you'll be well-equipped to leverage ${topic} in your own projects.

Remember, the key to success with ${topic} is continuous learning and experimentation. Stay curious, keep building, and don't hesitate to explore new possibilities!`,

        `The landscape of ${topic} is constantly evolving, and staying ahead of the curve requires a deep understanding of both current trends and emerging technologies. In this article, we'll examine the latest developments and provide actionable insights for developers looking to enhance their skills.

## Current State of ${topic}

Today's ${topic} ecosystem is more mature and feature-rich than ever before. With improved tooling, better documentation, and a growing community, developers have access to resources that make implementation more straightforward.

## Emerging Trends

Several key trends are shaping the future of ${topic}:

- **Automation**: Increasing focus on automated workflows and CI/CD integration
- **Performance**: New optimization techniques that improve speed and efficiency
- **Security**: Enhanced security features and best practices
- **Scalability**: Better support for large-scale deployments

## Industry Impact

The adoption of ${topic} across various industries has been remarkable:

- **Technology Sector**: 85% of tech companies have implemented ${topic} solutions
- **Financial Services**: Improved transaction processing and risk management
- **Healthcare**: Enhanced patient data management and analysis
- **E-commerce**: Better customer experience and operational efficiency

## Implementation Strategies

When implementing ${topic}, consider these strategic approaches:

1. **Phased Rollout**: Start with pilot projects before full deployment
2. **Team Training**: Invest in comprehensive team education
3. **Monitoring**: Implement robust monitoring and alerting systems
4. **Documentation**: Maintain detailed documentation for future reference

## Future Outlook

Looking ahead, ${topic} is poised for continued growth and innovation. Key areas to watch include:

- Advanced AI integration
- Improved developer experience
- Enhanced security features
- Better cross-platform compatibility

The future of ${topic} is bright, and developers who invest in learning these technologies now will be well-positioned for success in the years to come.`,

        `Building robust applications with ${topic} requires more than just technical knowledge—it demands a strategic approach to architecture, design, and implementation. In this deep dive, we'll explore advanced techniques that will elevate your ${topic} development skills.

## Architecture Patterns

Effective ${topic} applications rely on well-established architectural patterns:

### Microservices Architecture
Breaking down monolithic applications into smaller, manageable services provides better scalability and maintainability.

### Event-Driven Design
Implementing event-driven patterns enables better decoupling and improved system responsiveness.

### Domain-Driven Design
Organizing code around business domains leads to more maintainable and understandable applications.

## Performance Optimization

Optimizing ${topic} applications requires attention to multiple factors:

- **Caching Strategies**: Implement intelligent caching to reduce database load
- **Resource Management**: Optimize memory usage and CPU utilization
- **Network Optimization**: Minimize network overhead and latency
- **Database Tuning**: Optimize queries and indexing strategies

## Testing Strategies

Comprehensive testing is essential for reliable ${topic} applications:

\`\`\`javascript
// Example test structure
describe('${topic} Component', () => {
  it('should handle basic functionality', () => {
    const component = new ${topic}Component();
    expect(component.process()).toBeDefined();
  });
  
  it('should handle edge cases', () => {
    const component = new ${topic}Component();
    expect(() => component.process(null)).not.toThrow();
  });
});
\`\`\`

## Monitoring and Observability

Implementing comprehensive monitoring helps identify issues before they impact users:

- **Application Metrics**: Track performance indicators and user behavior
- **Error Tracking**: Monitor and alert on application errors
- **Log Analysis**: Centralize and analyze application logs
- **Health Checks**: Implement automated health monitoring

## Security Considerations

Security should be a primary concern when developing ${topic} applications:

1. **Input Validation**: Always validate and sanitize user inputs
2. **Authentication**: Implement robust authentication mechanisms
3. **Authorization**: Control access to resources and functionality
4. **Data Protection**: Encrypt sensitive data both in transit and at rest

## Deployment Strategies

Successful deployment requires careful planning and execution:

- **Blue-Green Deployment**: Minimize downtime during updates
- **Canary Releases**: Gradually roll out changes to a subset of users
- **Feature Flags**: Control feature availability without code changes
- **Rollback Procedures**: Prepare for quick rollback in case of issues

## Conclusion

Mastering ${topic} development is an ongoing journey that requires continuous learning and adaptation. By implementing these advanced techniques and best practices, you'll build more robust, scalable, and maintainable applications.

The key to success lies in understanding not just the technical aspects, but also the business context and user needs. Keep experimenting, stay updated with the latest developments, and always prioritize code quality and user experience.`
      ]
      
      const content = contentTemplates[Math.floor(Math.random() * contentTemplates.length)]
      
      blogPosts.push({
        title,
        content,
        author,
        publish_date: publishDate,
        tags: postTags
      })
    }
    
    return blogPosts
  }

  const loadSampleData = (sampleType) => {
    const samples = {
      blogposts: generateBlogPosts(),
      twitter: [
        {
          post_id: "tweet_1234567890",
          author: "@techinfluencer",
          author_id: "user_tech_001",
          content: "Just launched our new AI-powered database! 🚀 Real-time ingestion, automatic schema mapping, and zero-config setup. Check it out at folddb.io #database #AI #opensource",
          timestamp: "2024-10-21T14:32:00Z",
          likes: 342,
          retweets: 89,
          replies: 23,
          views: 12453,
          media: [
            {
              type: "image",
              url: "https://cdn.example.com/img1.jpg",
              alt: "FoldDB Dashboard Screenshot"
            }
          ],
          mentions: ["@opensource", "@devtools"],
          hashtags: ["database", "AI", "opensource"],
          reply_to: null,
          thread_position: 1,
          engagement_rate: 0.034
        },
        {
          post_id: "tweet_1234567891",
          author: "@datascientist_pro",
          author_id: "user_ds_042",
          content: "Amazing work @techinfluencer! Been testing FoldDB for the past week. The automatic schema inference saved us hours of setup time. Here are my benchmarks:",
          timestamp: "2024-10-21T15:18:00Z",
          likes: 156,
          retweets: 34,
          replies: 12,
          views: 5621,
          media: [
            {
              type: "image",
              url: "https://cdn.example.com/benchmark.png",
              alt: "Performance Benchmarks"
            }
          ],
          mentions: ["@techinfluencer"],
          hashtags: ["database", "performance"],
          reply_to: "tweet_1234567890",
          thread_position: null,
          engagement_rate: 0.036
        }
      ],
      instagram: [
        {
          post_id: "ig_post_987654321",
          username: "foodie_adventures",
          user_id: "ig_user_food_123",
          caption: "Best ramen in Tokyo! 🍜✨ The broth was simmering for 48 hours and you can taste every minute of it. Swipe for more pics! #tokyo #ramen #foodie #japan #travel",
          posted_at: "2024-10-20T09:45:00Z",
          location: {
            name: "Ichiran Ramen Shibuya",
            city: "Tokyo",
            country: "Japan",
            coordinates: {
              lat: 35.6595,
              lng: 139.7004
            }
          },
          media: [
            {
              type: "image",
              url: "https://cdn.instagram.example.com/ramen1.jpg",
              width: 1080,
              height: 1350,
              filter: "Valencia"
            },
            {
              type: "image",
              url: "https://cdn.instagram.example.com/ramen2.jpg",
              width: 1080,
              height: 1350,
              filter: "Valencia"
            },
            {
              type: "image",
              url: "https://cdn.instagram.example.com/ramen3.jpg",
              width: 1080,
              height: 1350,
              filter: "Valencia"
            }
          ],
          likes: 8234,
          comments_count: 456,
          saves: 892,
          shares: 234,
          hashtags: ["tokyo", "ramen", "foodie", "japan", "travel"],
          tagged_users: ["@ramen_tokyo_guide", "@japan_food_official"],
          comments: [
            {
              comment_id: "ig_comment_111",
              username: "tokyo_foodie",
              text: "Omg I was there last week! The tonkotsu broth is incredible 😍",
              timestamp: "2024-10-20T10:12:00Z",
              likes: 45
            },
            {
              comment_id: "ig_comment_112",
              username: "ramen_lover_88",
              text: "Adding this to my Tokyo bucket list! 📝",
              timestamp: "2024-10-20T11:30:00Z",
              likes: 23
            }
          ]
        },
        {
          post_id: "ig_post_987654322",
          username: "fitness_journey_2024",
          user_id: "ig_user_fit_456",
          caption: "Day 287 of my fitness journey! 💪 Down 45 lbs and feeling stronger than ever. Remember: progress > perfection. What's your fitness goal? #fitness #transformation #motivation #workout",
          posted_at: "2024-10-21T06:00:00Z",
          location: {
            name: "Gold's Gym",
            city: "Los Angeles",
            country: "USA",
            coordinates: {
              lat: 34.0522,
              lng: -118.2437
            }
          },
          media: [
            {
              type: "video",
              url: "https://cdn.instagram.example.com/workout_vid.mp4",
              thumbnail: "https://cdn.instagram.example.com/workout_thumb.jpg",
              duration: 45,
              width: 1080,
              height: 1920
            }
          ],
          likes: 15672,
          comments_count: 892,
          saves: 2341,
          shares: 567,
          hashtags: ["fitness", "transformation", "motivation", "workout"],
          tagged_users: ["@personal_trainer_mike"],
          comments: [
            {
              comment_id: "ig_comment_113",
              username: "motivation_daily",
              text: "Incredible transformation! You're an inspiration! 🔥",
              timestamp: "2024-10-21T06:15:00Z",
              likes: 234
            }
          ]
        }
      ],
      linkedin: [
        {
          post_id: "li_post_555666777",
          author: {
            name: "Sarah Chen",
            title: "CTO at TechVentures Inc.",
            profile_url: "linkedin.com/in/sarah-chen-cto",
            user_id: "li_user_sarah_123"
          },
          content: "Excited to announce that our team has successfully migrated our entire data infrastructure to a real-time event-driven architecture! 🎉\n\nKey achievements:\n• 10x reduction in data latency (from 5 minutes to 30 seconds)\n• 40% cost savings on infrastructure\n• Improved data quality through automated validation\n• Seamless integration with our ML pipelines\n\nHuge shoutout to the engineering team for their incredible work over the past 6 months. This wouldn't have been possible without their dedication and expertise.\n\nHappy to share more details for anyone interested in event-driven architectures. Feel free to reach out!\n\n#DataEngineering #EventDriven #TechLeadership #Innovation",
          posted_at: "2024-10-21T13:00:00Z",
          article: null,
          media: [
            {
              type: "document",
              title: "Event-Driven Architecture: Our Journey",
              url: "https://cdn.linkedin.example.com/architecture_diagram.pdf",
              pages: 12
            }
          ],
          reactions: {
            like: 1247,
            celebrate: 342,
            support: 89,
            insightful: 156,
            love: 67
          },
          comments_count: 87,
          reposts: 234,
          comments: [
            {
              comment_id: "li_comment_aaa111",
              author: {
                name: "Michael Roberts",
                title: "Senior Data Engineer at DataCorp",
                user_id: "li_user_mike_456"
              },
              text: "Congratulations Sarah! We're looking at a similar migration. Would love to connect and learn from your experience.",
              timestamp: "2024-10-21T13:45:00Z",
              reactions: {
                like: 45
              }
            },
            {
              comment_id: "li_comment_aaa112",
              author: {
                name: "Jennifer Liu",
                title: "VP Engineering at CloudScale",
                user_id: "li_user_jen_789"
              },
              text: "Impressive results! The 10x latency improvement is remarkable. Did you use Apache Kafka or another streaming platform?",
              timestamp: "2024-10-21T14:20:00Z",
              reactions: {
                like: 23,
                insightful: 8
              }
            }
          ],
          industries: ["Technology", "Data Engineering", "Cloud Computing"],
          skills_mentioned: ["Event-Driven Architecture", "Data Engineering", "ML Pipeline", "Infrastructure"]
        },
        {
          post_id: "li_post_555666778",
          author: {
            name: "Marcus Thompson",
            title: "Product Manager | Ex-Google | Building the Future of Work",
            profile_url: "linkedin.com/in/marcus-thompson-pm",
            user_id: "li_user_marcus_234"
          },
          content: "5 lessons from shipping 100+ product features:\n\n1. Talk to users BEFORE writing specs\n2. Small iterations > big launches\n3. Metrics don't tell the whole story\n4. Technical debt is real debt\n5. Celebrate wins with your team\n\nWhat would you add to this list?\n\n#ProductManagement #Technology #Leadership",
          posted_at: "2024-10-21T10:30:00Z",
          article: null,
          media: [],
          reactions: {
            like: 3421,
            celebrate: 892,
            insightful: 567,
            love: 234
          },
          comments_count: 234,
          reposts: 789,
          comments: [],
          industries: ["Product Management", "Technology", "Startups"],
          skills_mentioned: ["Product Management", "User Research", "Agile"]
        }
      ],
      tiktok: [
        {
          video_id: "tt_vid_777888999",
          username: "coding_tips_daily",
          user_id: "tt_user_code_001",
          caption: "3 JavaScript array methods that will blow your mind 🤯 #coding #javascript #programming #webdev #learntocode",
          posted_at: "2024-10-21T16:45:00Z",
          video: {
            url: "https://cdn.tiktok.example.com/video_js_tips.mp4",
            thumbnail: "https://cdn.tiktok.example.com/thumb_js_tips.jpg",
            duration: 58,
            width: 1080,
            height: 1920,
            format: "mp4"
          },
          audio: {
            title: "Epic Tech Music",
            artist: "TechBeats Production",
            audio_id: "audio_tech_123"
          },
          statistics: {
            views: 2834562,
            likes: 342891,
            comments: 12453,
            shares: 45672,
            saves: 89234,
            completion_rate: 0.78
          },
          hashtags: ["coding", "javascript", "programming", "webdev", "learntocode"],
          mentions: [],
          effects: ["Green Screen", "Text Animation", "Transition Effect"],
          comments: [
            {
              comment_id: "tt_comment_xyz1",
              username: "dev_beginner_22",
              text: "Just used .reduce() in my project and it worked perfectly! Thanks!",
              timestamp: "2024-10-21T17:00:00Z",
              likes: 1234,
              replies_count: 45
            },
            {
              comment_id: "tt_comment_xyz2",
              username: "senior_dev_10yrs",
              text: "Great explanation! Would love to see more advanced array methods",
              timestamp: "2024-10-21T17:30:00Z",
              likes: 892,
              replies_count: 23
            }
          ]
        },
        {
          video_id: "tt_vid_777889000",
          username: "travel_with_emma",
          user_id: "tt_user_travel_042",
          caption: "POV: You visit Santorini for the first time 🇬🇷✨ #travel #santorini #greece #traveltok #wanderlust",
          posted_at: "2024-10-20T08:20:00Z",
          video: {
            url: "https://cdn.tiktok.example.com/video_santorini.mp4",
            thumbnail: "https://cdn.tiktok.example.com/thumb_santorini.jpg",
            duration: 43,
            width: 1080,
            height: 1920,
            format: "mp4"
          },
          audio: {
            title: "Summer Vibes",
            artist: "Chill Beats Co.",
            audio_id: "audio_summer_456"
          },
          statistics: {
            views: 8923451,
            likes: 1234567,
            comments: 34521,
            shares: 123456,
            saves: 234567,
            completion_rate: 0.92
          },
          hashtags: ["travel", "santorini", "greece", "traveltok", "wanderlust"],
          mentions: ["@visit_greece_official"],
          effects: ["Color Grading", "Slow Motion", "Zoom Transition"],
          location: {
            name: "Santorini",
            country: "Greece",
            coordinates: {
              lat: 36.3932,
              lng: 25.4615
            }
          },
          comments: [
            {
              comment_id: "tt_comment_xyz3",
              username: "greece_lover_89",
              text: "Adding this to my 2025 bucket list! 😍",
              timestamp: "2024-10-20T09:00:00Z",
              likes: 4521,
              replies_count: 234
            }
          ]
        }
      ]
    }
    
    setJsonData(JSON.stringify(samples[sampleType], null, 2))
  }

  return (
    <div className="space-y-4">
      {/* Status Bar */}
      {ingestionStatus && (
        <div className="card-terminal p-3 border-l-4 border-terminal-green">
          <div className="flex items-center gap-4 text-sm">
            <span className={`badge-terminal ${
              ingestionStatus.enabled && ingestionStatus.configured 
                ? 'badge-terminal-success' 
                : 'badge-terminal-error'
            }`}>
              {ingestionStatus.enabled && ingestionStatus.configured ? 'Ready' : 'Not Configured'}
            </span>
            <span className="text-terminal-dim">{ingestionStatus.provider} · {ingestionStatus.model}</span>
            <span className="text-xs text-terminal-dim">Configure AI settings using the Settings button in the header</span>
          </div>
        </div>
      )}


      <div className="card-terminal p-4">
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-terminal-green font-medium">
            <span className="text-terminal-dim">$</span> JSON Data
          </h3>
          <div className="flex gap-2">
            <button
              onClick={() => loadSampleData('blogposts')}
              className="btn-terminal text-xs py-1 px-3"
            >
              Blog Posts (100)
            </button>
            <button
              onClick={() => loadSampleData('twitter')}
              className="btn-terminal text-xs py-1 px-3"
            >
              Twitter
            </button>
            <button
              onClick={() => loadSampleData('instagram')}
              className="btn-terminal text-xs py-1 px-3"
            >
              Instagram
            </button>
            <button
              onClick={() => loadSampleData('linkedin')}
              className="btn-terminal text-xs py-1 px-3"
            >
              LinkedIn
            </button>
            <button
              onClick={() => loadSampleData('tiktok')}
              className="btn-terminal text-xs py-1 px-3"
            >
              TikTok
            </button>
          </div>
        </div>
        
        <textarea
          id="jsonData"
          value={jsonData}
          onChange={(e) => setJsonData(e.target.value)}
          placeholder="Enter your JSON data here or load a sample..."
          className="textarea-terminal w-full h-64"
        />
      </div>

      {/* Process Button */}
      <div className="card-terminal p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <label className="flex items-center gap-2 text-sm cursor-pointer">
              <input
                type="checkbox"
                checked={autoExecute}
                onChange={(e) => setAutoExecute(e.target.checked)}
                className="w-4 h-4 accent-terminal-green bg-terminal border-terminal"
              />
              <span className="text-terminal">Auto-execute mutations</span>
            </label>
            <span className="text-xs text-terminal-dim">AI will analyze and automatically map data to schemas</span>
          </div>
          
          <button
            onClick={processIngestion}
            disabled={isLoading || !jsonData.trim()}
            className={`btn-terminal px-6 py-2.5 font-medium ${
              isLoading || !jsonData.trim()
                ? 'opacity-50 cursor-not-allowed'
                : 'btn-terminal-primary'
            }`}
          >
            {isLoading ? (
              <>
                <span className="spinner-terminal"></span>
                <span>Processing...</span>
              </>
            ) : (
              <>
                <span>→</span>
                <span>Process Data</span>
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  )
}

export default IngestionTab
